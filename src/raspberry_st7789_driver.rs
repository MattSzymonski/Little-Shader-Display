use std::error::Error;
use std::thread;
use std::time::Duration;

use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{Point, RgbColor};
use rppal::gpio::Gpio;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
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
        
        let spi = RaspberrySpi::new(Spi::new(Bus::Spi0, SlaveSelect::Ss0, 64_000_000, Mode::Mode0)?);
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
    
    pub fn draw_raw(&mut self, image_bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        // Convert RGBA8888 to RGB565 (LE packed bytes)
        let rgb565_bytes = rgba8888_to_rgb565_u8(image_bytes);
    
        // Compute square side size from byte count
        let dim = (rgb565_bytes.len() / 2) as u32;
        let side = (dim as f32).sqrt() as u32;
    
        let raw_image: ImageRawLE<Rgb565> = ImageRawLE::new(&rgb565_bytes, side);
        let image = Image::new(&raw_image, Point::new(-50, 60));
        
        image.draw(&mut self.display);
        Ok(())
    }

}

// Converts RGBA8888 (4 bytes per pixel) to RGB565 (2 bytes per pixel, little-endian)
// Skips the alpha channel entirely.
fn rgba8888_to_rgb565_u8(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity((input.len() / 4) * 2); // 2 bytes per pixel (RGB565)

    for chunk in input.chunks_exact(4) {
        let rgb565: u16 =
            ((chunk[0] as u16 & 0xF8) << 8) | // Red: upper 5 bits
            ((chunk[1] as u16 & 0xFC) << 3) | // Green: upper 6 bits
            ((chunk[2] as u16) >> 3);         // Blue: upper 5 bits

        output.push((rgb565 & 0xFF) as u8);      // Low byte
        output.push((rgb565 >> 8) as u8);        // High byte
    }

    output
}