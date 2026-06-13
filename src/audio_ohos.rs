// OHOS 远端音频播放：thin FFI 桥到 libohaudio.so。
//
// 路径：rust 收到 audio frame → opus 解码 (在 client.rs::AudioHandler) → f32 PCM
// → append_pcm 推到 ringbuf → OH_AudioRenderer_OnWriteDataCallback 拉 PCM 填给
// OHOS 系统 buffer。Callback 在 OS 音频线程跑，跟我们 push pcm 的 worker 线程
// 不同步，靠 ringbuf 的内部锁解决。
//
// 参考：
// * NDK 头 <ohaudio/native_audiostreambuilder.h>、<ohaudio/native_audiorenderer.h>
// * OHOS 文档 media/audio/using-ohaudio-for-playback.md
//
// 注意：OH_AudioRenderer 在 ringbuf 数据不够时会拉 0；callback 必须始终返回
// AUDIO_DATA_CALLBACK_RESULT_VALID（0），否则 OS 认为是错误数据会丢弃整个
// buffer 引发杂音。

#![cfg(target_env = "ohos")]

use hbb_common::{anyhow::anyhow, log, ResultType};
use ringbuf::{ring_buffer::RbBase, HeapRb, Rb};
use std::{
    os::raw::{c_int, c_void},
    sync::{Arc, Mutex},
};

// ===== libohaudio.so FFI =====

#[allow(non_camel_case_types)]
pub enum OH_AudioStreamBuilder {}
#[allow(non_camel_case_types)]
pub enum OH_AudioRenderer {}

#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
#[derive(Clone, Copy)]
pub enum OH_AudioStream_Type {
    Renderer = 1,
    Capturer = 2,
}

#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
#[derive(Clone, Copy)]
pub enum OH_AudioStream_EncodingType {
    Raw = 0,
    AudioVivid = 1,
    EAc3 = 2,
}

#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
#[derive(Clone, Copy)]
pub enum OH_AudioStream_SampleFormat {
    U8 = 0,
    S16Le = 1,
    S24Le = 2,
    S32Le = 3,
    F32Le = 4,
}

#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
#[derive(Clone, Copy)]
pub enum OH_AudioStream_Usage {
    Unknown = 0,
    Music = 1,
    VoiceCommunication = 2,
    Movie = 10,
    Game = 11,
    VideoCommunication = 17,
}

#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OH_AudioStream_Result {
    Success = 0,
    ErrorInvalidParam = 1,
    ErrorIllegalState = 2,
    ErrorSystem = 3,
}

#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
#[derive(Clone, Copy)]
pub enum OH_AudioData_Callback_Result {
    Invalid = -1,
    Valid = 0,
}

pub type OH_AudioRenderer_OnWriteDataCallback = unsafe extern "C" fn(
    renderer: *mut OH_AudioRenderer,
    user_data: *mut c_void,
    audio_data: *mut c_void,
    audio_data_size: c_int,
) -> OH_AudioData_Callback_Result;

#[link(name = "ohaudio")]
extern "C" {
    fn OH_AudioStreamBuilder_Create(
        builder: *mut *mut OH_AudioStreamBuilder,
        stream_type: OH_AudioStream_Type,
    ) -> OH_AudioStream_Result;
    fn OH_AudioStreamBuilder_Destroy(builder: *mut OH_AudioStreamBuilder)
        -> OH_AudioStream_Result;
    fn OH_AudioStreamBuilder_SetSamplingRate(
        builder: *mut OH_AudioStreamBuilder,
        rate: i32,
    ) -> OH_AudioStream_Result;
    fn OH_AudioStreamBuilder_SetChannelCount(
        builder: *mut OH_AudioStreamBuilder,
        channel_count: i32,
    ) -> OH_AudioStream_Result;
    fn OH_AudioStreamBuilder_SetSampleFormat(
        builder: *mut OH_AudioStreamBuilder,
        format: OH_AudioStream_SampleFormat,
    ) -> OH_AudioStream_Result;
    fn OH_AudioStreamBuilder_SetEncodingType(
        builder: *mut OH_AudioStreamBuilder,
        encoding: OH_AudioStream_EncodingType,
    ) -> OH_AudioStream_Result;
    fn OH_AudioStreamBuilder_SetRendererInfo(
        builder: *mut OH_AudioStreamBuilder,
        usage: OH_AudioStream_Usage,
    ) -> OH_AudioStream_Result;
    fn OH_AudioStreamBuilder_SetRendererWriteDataCallback(
        builder: *mut OH_AudioStreamBuilder,
        callback: OH_AudioRenderer_OnWriteDataCallback,
        user_data: *mut c_void,
    ) -> OH_AudioStream_Result;
    fn OH_AudioStreamBuilder_GenerateRenderer(
        builder: *mut OH_AudioStreamBuilder,
        renderer: *mut *mut OH_AudioRenderer,
    ) -> OH_AudioStream_Result;
    fn OH_AudioRenderer_Start(renderer: *mut OH_AudioRenderer) -> OH_AudioStream_Result;
    fn OH_AudioRenderer_Stop(renderer: *mut OH_AudioRenderer) -> OH_AudioStream_Result;
    fn OH_AudioRenderer_Release(renderer: *mut OH_AudioRenderer) -> OH_AudioStream_Result;
}

// ===== rust 侧封装 =====

/// 跨线程共享的 PCM ringbuf：worker 推 push，OS audio 线程在 callback 里 pop。
type Ringbuf = Arc<Mutex<HeapRb<f32>>>;

/// callback 的 user_data。生命周期：跟 OhosAudioRenderer 同生死，由 Box::into_raw
/// 暴露给 C 端，stop() 时 Box::from_raw 回收。**绝不能** Box::leak / 中途 drop。
struct CallbackCtx {
    rb: Ringbuf,
}

pub struct OhosAudioRenderer {
    sample_rate: u32,
    channels: u16,
    rb: Ringbuf,
    builder: *mut OH_AudioStreamBuilder,
    renderer: *mut OH_AudioRenderer,
    ctx_ptr: *mut CallbackCtx,
    started: bool,
}

// libohaudio.so 的所有 handle 仅在 OhosAudioRenderer 拥有的线程上下文里被释放，
// `*mut OH_AudioStreamBuilder/Renderer` 本身可跨线程移动（OS 那边线程安全）。
unsafe impl Send for OhosAudioRenderer {}
unsafe impl Sync for OhosAudioRenderer {}

const RING_MS: usize = 1500;

impl OhosAudioRenderer {
    pub fn new(sample_rate: u32, channels: u16) -> ResultType<Self> {
        if channels == 0 || channels > 8 {
            return Err(anyhow!("invalid channel count {channels}"));
        }
        let capacity = (sample_rate as usize) * (channels as usize) * RING_MS / 1000;
        let rb: Ringbuf = Arc::new(Mutex::new(HeapRb::<f32>::new(capacity)));

        let mut builder: *mut OH_AudioStreamBuilder = std::ptr::null_mut();
        check(
            unsafe {
                OH_AudioStreamBuilder_Create(&mut builder, OH_AudioStream_Type::Renderer)
            },
            "AudioStreamBuilder_Create",
        )?;
        check(
            unsafe { OH_AudioStreamBuilder_SetSamplingRate(builder, sample_rate as i32) },
            "SetSamplingRate",
        )?;
        check(
            unsafe { OH_AudioStreamBuilder_SetChannelCount(builder, channels as i32) },
            "SetChannelCount",
        )?;
        check(
            unsafe {
                OH_AudioStreamBuilder_SetSampleFormat(
                    builder,
                    OH_AudioStream_SampleFormat::F32Le,
                )
            },
            "SetSampleFormat",
        )?;
        check(
            unsafe {
                OH_AudioStreamBuilder_SetEncodingType(
                    builder,
                    OH_AudioStream_EncodingType::Raw,
                )
            },
            "SetEncodingType",
        )?;
        check(
            unsafe {
                OH_AudioStreamBuilder_SetRendererInfo(builder, OH_AudioStream_Usage::Music)
            },
            "SetRendererInfo",
        )?;

        let ctx = Box::new(CallbackCtx { rb: rb.clone() });
        let ctx_ptr = Box::into_raw(ctx);
        check(
            unsafe {
                OH_AudioStreamBuilder_SetRendererWriteDataCallback(
                    builder,
                    on_write_data,
                    ctx_ptr as *mut c_void,
                )
            },
            "SetRendererWriteDataCallback",
        )?;

        let mut renderer: *mut OH_AudioRenderer = std::ptr::null_mut();
        check(
            unsafe { OH_AudioStreamBuilder_GenerateRenderer(builder, &mut renderer) },
            "GenerateRenderer",
        )?;

        log::info!(
            "[ohaudio] renderer created sample_rate={sample_rate} channels={channels}"
        );

        Ok(Self {
            sample_rate,
            channels,
            rb,
            builder,
            renderer,
            ctx_ptr,
            started: false,
        })
    }

    pub fn start(&mut self) -> ResultType<()> {
        if self.started {
            return Ok(());
        }
        check(unsafe { OH_AudioRenderer_Start(self.renderer) }, "Renderer_Start")?;
        self.started = true;
        log::info!("[ohaudio] renderer started");
        Ok(())
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// 推 PCM 到 ringbuf。同 client.rs::AudioBuffer::append_pcm 的语义：
    /// buffer 满时直接 overwrite 旧数据（音频低延迟优先于完整性）。
    pub fn append_pcm(&self, buffer: &[f32]) {
        let mut rb = self.rb.lock().unwrap();
        let cap = rb.capacity();
        if buffer.len() > cap {
            rb.push_slice_overwrite(buffer);
            return;
        }
        let want = rb.occupied_len() + buffer.len();
        if want > cap {
            rb.skip(want - cap);
        }
        rb.push_slice_overwrite(buffer);
    }
}

impl Drop for OhosAudioRenderer {
    fn drop(&mut self) {
        unsafe {
            if self.started {
                let _ = OH_AudioRenderer_Stop(self.renderer);
            }
            if !self.renderer.is_null() {
                let _ = OH_AudioRenderer_Release(self.renderer);
            }
            if !self.builder.is_null() {
                let _ = OH_AudioStreamBuilder_Destroy(self.builder);
            }
            if !self.ctx_ptr.is_null() {
                drop(Box::from_raw(self.ctx_ptr));
            }
        }
        log::info!("[ohaudio] renderer released");
    }
}

fn check(r: OH_AudioStream_Result, ctx: &str) -> ResultType<()> {
    if r == OH_AudioStream_Result::Success {
        Ok(())
    } else {
        Err(anyhow!("{ctx} failed: {:?}", r as i32))
    }
}

/// OS audio 线程调入。从 ringbuf 拉 f32 PCM 填进 audio_data；不足填 0。
/// 必须返回 Valid(0) 否则 OS 丢整块 buffer。
unsafe extern "C" fn on_write_data(
    _renderer: *mut OH_AudioRenderer,
    user_data: *mut c_void,
    audio_data: *mut c_void,
    audio_data_size: c_int,
) -> OH_AudioData_Callback_Result {
    let ctx = &*(user_data as *const CallbackCtx);
    let nf32 = (audio_data_size as usize) / std::mem::size_of::<f32>();
    let dst = std::slice::from_raw_parts_mut(audio_data as *mut f32, nf32);
    let mut rb = ctx.rb.lock().unwrap();
    // ringbuf 0.3 `pop_slice` 会在 dst 比 occupied 多时 panic，必须先量。
    let take = std::cmp::min(rb.occupied_len(), nf32);
    if take > 0 {
        rb.pop_slice(&mut dst[..take]);
    }
    // 不够喂的部分静音填，避免 callback 不写完整 buffer 引起爆音。
    for x in dst[take..].iter_mut() {
        *x = 0.0;
    }
    OH_AudioData_Callback_Result::Valid
}
