use alloc::boxed::Box;

use embedded_hal::blocking::spi::Write;
use embedded_hal::digital::v2::OutputPin;

pub struct Display {
    buffer: [u8; 2048],
    cs: Box<dyn OutputPin<Error = !>>,
    disp: Box<dyn OutputPin<Error = !>>,
}

impl Display {
    pub fn new(cs: Box<dyn OutputPin<Error = !>>, disp: Box<dyn OutputPin<Error = !>>) -> Display {
        Display {
            buffer: [0; 2048],
            cs,
            disp,
        }
    }

    pub fn clear<SPI, SPIE>(mut self, spi: &mut SPI) -> Result<(), SPIE>
    where
        SPI: Write<u8, Error = SPIE>,
    {
        self.cs.set_high().unwrap();
        spi.write(&[0x20, 0x00])?;
        self.cs.set_low().unwrap();

        Ok(())
    }

    pub fn write_line<SPI, SPIE>(
        mut self,
        spi: &mut SPI,
        address: u8,
        buffer: [u8; 16],
    ) -> Result<(), SPIE>
    where
        SPI: Write<u8, Error = SPIE>,
    {
        self.cs.set_high().unwrap();
        spi.write(&[0x08, address])?;
        spi.write(&buffer)?;
        spi.write(&[0x00, 0x00])?;
        self.cs.set_low().unwrap();

        Ok(())
    }
}
