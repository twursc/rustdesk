// OHOS v0.1 主控端：不需要真正的 capture，只为满足 codec/convert 模块对
// Capturer/PixelBuffer/Display 的类型/符号引用。所有方法在被调用时 panic/Err。

use crate::{Frame, Pixfmt};
use std::{io, time::Duration};

pub struct Capturer {
    display: Display,
}

impl Capturer {
    pub fn new(display: Display) -> io::Result<Capturer> {
        Ok(Capturer { display })
    }

    pub fn width(&self) -> usize {
        self.display.width()
    }

    pub fn height(&self) -> usize {
        self.display.height()
    }
}

impl crate::TraitCapturer for Capturer {
    fn frame<'a>(&'a mut self, _timeout: Duration) -> io::Result<Frame<'a>> {
        // OHOS 控制端不做屏幕捕获；v0.2 会用 AVScreenCapture 实现。
        Err(io::ErrorKind::Unsupported.into())
    }
}

pub struct PixelBuffer<'a> {
    data: &'a [u8],
    width: usize,
    height: usize,
    stride: Vec<usize>,
}

impl<'a> PixelBuffer<'a> {
    pub fn new(data: &'a [u8], width: usize, height: usize) -> Self {
        let stride0 = if height == 0 { 0 } else { data.len() / height.max(1) };
        let stride = vec![stride0];
        PixelBuffer { data, width, height, stride }
    }
}

impl<'a> crate::TraitPixelBuffer for PixelBuffer<'a> {
    fn data(&self) -> &[u8] { self.data }
    fn width(&self) -> usize { self.width }
    fn height(&self) -> usize { self.height }
    fn stride(&self) -> Vec<usize> { self.stride.clone() }
    fn pixfmt(&self) -> Pixfmt { Pixfmt::RGBA }
}

pub struct Display {
    default: bool,
    w: usize,
    h: usize,
}

impl Display {
    pub fn primary() -> io::Result<Display> {
        Ok(Display { default: true, w: 0, h: 0 })
    }
    pub fn all() -> io::Result<Vec<Display>> {
        Ok(vec![Display::primary()?])
    }
    pub fn width(&self) -> usize { self.w }
    pub fn height(&self) -> usize { self.h }
    pub fn origin(&self) -> (i32, i32) { (0, 0) }
    pub fn is_online(&self) -> bool { true }
    pub fn is_primary(&self) -> bool { self.default }
    pub fn name(&self) -> String { "OHOS".into() }
    pub fn refresh_size() {}
    pub fn fix_quality() -> u16 { 1 }
}

pub fn is_start() -> Option<bool> { Some(false) }
