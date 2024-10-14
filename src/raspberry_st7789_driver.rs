use std::error::Error;
use std::thread;
use std::time::Duration;

use display_interface_spi::SPIInterface;
use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{Point, RgbColor, Transform};
use embedded_graphics::primitives::{Circle, PrimitiveStyle};
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::models::ST7789;
use mipidsi::options::ColorInversion;
use mipidsi::Builder;
use rppal::gpio::Gpio;
use rppal::hal::Delay;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use rppal::system::DeviceInfo;
use rppal::gpio::OutputPin as RppalOutputPin;
use rppal::gpio::Error as RppalError;
use rppal::spi::{Error as SpiError};
use embedded_graphics::Drawable;
use embedded_graphics::draw_target::DrawTarget;

const DC_PIN_NUMBER: u8 = 25;
const RST_PIN_NUMBER: u8 = 27;
const CS_PIN_NUMBER: u8 = 8;
const BL_PIN_NUMBER: u8 = 18;


struct NoCs;

impl embedded_hal::digital::OutputPin for NoCs {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl embedded_hal::digital::ErrorType for NoCs {
    type Error = core::convert::Infallible;
}
pub struct RaspberryST7789Driver {
    display: mipidsi::Display<SPIInterface<ExclusiveDevice<Spi, NoCs, embedded_hal_bus::spi::NoDelay>, RppalOutputPin>, ST7789, mipidsi::NoResetPin>,
}

impl RaspberryST7789Driver {
    pub fn new() -> Result<Self, Box<dyn Error>>  {
        let gpio = Gpio::new().unwrap();
        let dc_pin = gpio.get(DC_PIN_NUMBER)?.into_output();
        let rst_pin = gpio.get(RST_PIN_NUMBER)?.into_output();
        let cs_pin = gpio.get(CS_PIN_NUMBER)?.into_output();
        let bl_pin = gpio.get(BL_PIN_NUMBER)?.into_output();
        
        let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss1, 60_000_000_u32, Mode::Mode0).unwrap();
        let spi_device = ExclusiveDevice::new_no_delay(spi, NoCs).unwrap();
        let di = SPIInterface::new(spi_device, dc_pin);
        let mut delay = Delay::new();
        let mut display = Builder::new(ST7789, di)
            .display_size(240 as u16, 280 as u16)
            .invert_colors(ColorInversion::Inverted)
            .init(&mut delay)
            .unwrap();

            display.clear(Rgb565::BLACK).unwrap();
        Ok(Self {
            display,
        })    
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn Error>> {
        //self.display.init(&mut self.delay);
        //self.display.set_orientation(Orientation::Portrait);
        //self.display.  clear(Rgb565::BLACK);
        //self.display.flush()?;
        Ok(())
    }

    // Important: This code handles only 8-bit images with 3 channels! In photopea scale image then save it as raw with the same dimensions
    pub fn draw_raw(&mut self, image_bytes: &[u8], is_bgr: bool) -> Result<(), Box<dyn Error>> {
        let rgb565_data = pack_888_to_rgb565(&image_bytes, is_bgr); 
        let rgb565_data_split = rgb565_to_u8(&rgb565_data);
        let raw_image: ImageRawLE<Rgb565> = ImageRawLE::new(&rgb565_data_split, (rgb565_data.len() as f32).sqrt() as u32);
        let image = Image::new(&raw_image, Point::new(-30, 0));
        image.draw(&mut self.display);
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