use std::error::Error;
use std::thread;
use std::time::Duration;

use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{Point, RgbColor, Transform};
use embedded_graphics::primitives::{Circle, PrimitiveStyle};
use rppal::gpio::Gpio;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use rppal::system::DeviceInfo;
use st7789::{Orientation, ST7789};
use embedded_hal::digital::v2::OutputPin as EmbeddedOutputPin;
use rppal::gpio::OutputPin as RppalOutputPin;
use rppal::gpio::Error as RppalError;
use rppal::spi::{Error as SpiError};
use embedded_hal::blocking::spi::Write; 
use embedded_hal::blocking::delay::DelayUs; 
use embedded_graphics::Drawable;
use embedded_graphics::draw_target::DrawTarget;

const DC_PIN_NUMBER: u8 = 25;
const RST_PIN_NUMBER: u8 = 27;
const CS_PIN_NUMBER: u8 = 8;
const BL_PIN_NUMBER: u8 = 18;

pub struct RaspberryDelayOutputPin {
    pin: RppalOutputPin,
}

// Implement the OutputPin trait for the wrapper
impl RaspberryDelayOutputPin {
    pub fn new(pin: RppalOutputPin) -> Self {
        Self { pin }
    }
}

impl EmbeddedOutputPin for RaspberryDelayOutputPin {
    type Error = RppalError; // Using rppal's error type

    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(self.pin.set_low())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(self.pin.set_high())
    }

    fn set_state(&mut self, state: embedded_hal::digital::v2::PinState) -> Result<(), Self::Error> {
        match state {
            embedded_hal::digital::v2::PinState::Low => self.set_low(),
            embedded_hal::digital::v2::PinState::High => self.set_high(),
        }
    }
}

pub struct RaspberryDelay;

impl RaspberryDelay {
    pub fn new() -> Self {
        Self
    }
}

impl DelayUs<u32> for RaspberryDelay {
    fn delay_us(&mut self, us: u32) {
        thread::sleep(Duration::from_micros(us as u64));
    }
}

pub struct RaspberrySpi {
    spi: Spi,
}

impl RaspberrySpi {
    pub fn new(spi: Spi) -> Self {
        Self { spi }
    }
}

impl Write<u8> for RaspberrySpi {
    type Error = SpiError; 
    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.spi.write(words)?;
        Ok(())
    }
}

pub struct RaspberryST7789Driver {
    delay: RaspberryDelay,
    display: ST7789<SPIInterfaceNoCS<RaspberrySpi, RaspberryDelayOutputPin>, RaspberryDelayOutputPin>,
}

impl RaspberryST7789Driver {
    pub fn new() -> Result<Self, Box<dyn Error>>  {
        let gpio = Gpio::new()?; 
        let dc_pin = RaspberryDelayOutputPin::new(gpio.get(DC_PIN_NUMBER)?.into_output());
        let rst_pin = RaspberryDelayOutputPin::new(gpio.get(RST_PIN_NUMBER)?.into_output());
        let cs_pin = RaspberryDelayOutputPin::new(gpio.get(CS_PIN_NUMBER)?.into_output());
        let bl_pin = RaspberryDelayOutputPin::new(gpio.get(BL_PIN_NUMBER)?.into_output());
        
        let spi = RaspberrySpi::new(Spi::new(Bus::Spi0, SlaveSelect::Ss0, 16_000_000, Mode::Mode0)?);
        let display_interface = SPIInterfaceNoCS::new(spi, dc_pin);
        let display = ST7789::new(display_interface, rst_pin, 240, 280);
    
        Ok(Self {
            delay: RaspberryDelay::new(),
            display,
        })    
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn Error>> {
        self.display.init(&mut self.delay);
        self.display.set_orientation(Orientation::Portrait);
        self.display.clear(Rgb565::BLACK);
        //self.display.flush()?;
        Ok(())
    }

    // Important: This code handles only 8-bit images with 3 channels! In photopea scale image then save it as raw with the same dimensions
    pub fn draw_raw(&mut self, image_bytes: &[u8], is_bgr: bool) -> Result<(), Box<dyn Error>> {
        let rgb565_data = pack_888_to_rgb565(&image_bytes, is_bgr); // -15 FPS
        let rgb565_data_split = rgb565_to_u8(&rgb565_data); // -20 FPS
        let raw_image: ImageRawLE<Rgb565> = ImageRawLE::new(&rgb565_data_split, (rgb565_data.len() as f32).sqrt() as u32); // -2 FPS
        let image = Image::new(&raw_image, Point::new(-50, 0)); // -1 FPS
        image.draw(&mut self.display); // -80 FPS
        //self.display.flush()?;
        Ok(())
    }
}

// If brga is false then rgb is used
fn pack_888_to_rgb565(input: &[u8], is_bgr: bool) -> Vec<u16> {
    let mut output: Vec<u16> = Vec::new();
    
    for chunk in input.chunks(3) {
        let (r, g, b) = match chunk {
            [r, g, b] => { if is_bgr { (*b, *g, *r) } else {  (*r, *g, *b) } }
            [r, g] => { if is_bgr { (*g, 0, *r) } else { (*r, *g, 0) } }
            [r] => { if is_bgr { (0, 0, *r) } else { (*r, 0, 0)  } }
            _ => (0, 0, 0),  // Default case
        };
        
        // Convert 3 u8 color values to rgb565 packed into a single u16
        let r_5 = (r >> 3) & 0x1F;  // 5 bits for red
        let g_6 = (g >> 2) & 0x3F;  // 6 bits for green
        let b_5 = (b >> 3) & 0x1F;  // 5 bits for blue

        // Combine into a single 16-bit value
        let result = (((r_5 as u16) << 11) | ((g_6 as u16) << 5) | b_5 as u16);
        output.push(result);
    }

    output
}

fn rgb565_to_u8(rgb565_data: &[u16]) -> Vec<u8> {
    let mut byte_data = Vec::with_capacity(rgb565_data.len() * 2); // Each u16 will be converted to 2 u8 values
    for &rgb565 in rgb565_data {
        let lower_byte = (rgb565 & 0xFF) as u8; // Lower byte
        let upper_byte = (rgb565 >> 8) as u8; // Upper byte
        byte_data.push(lower_byte);
        byte_data.push(upper_byte);
    }

    byte_data
}