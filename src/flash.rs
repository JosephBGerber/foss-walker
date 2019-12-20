const SIZE: usize = 128;
const SECTOR_11: *mut u8 = 0x080E_0000 as *mut u8;

use stm32f4xx_hal::stm32::FLASH;

/// Interface for for manipulating flash sector 11
pub struct Flash {
    flash: FLASH,
    flash_sector: &'static mut [u8],
    buffer: [u8; SIZE],
}

impl Flash {
    /// Create a new Flash instance. This object requires sole ownership of
    /// the FLASH peripheral to safely operate.
    pub fn new(flash: FLASH) -> Flash {
        // Unlock the FLASH_CR register
        unsafe {
            flash.keyr.write(|w| w.bits(0x4567_0123));
            flash.keyr.write(|w| w.bits(0xCDEF_89AB));
        };

        // Create a buffer allowing random access to flash storage
        let mut buffer: [u8; SIZE] = [0; SIZE];

        // Create a slice pointing sector 11 of the flash module
        let flash_sector = unsafe { core::slice::from_raw_parts_mut(SECTOR_11, SIZE) };

        // Copy the data saved in the flash into the buffer
        for (index, byte) in flash_sector.iter().enumerate() {
            buffer[index] = *byte;
        }

        Flash {
            flash,
            flash_sector,
            buffer,
        }
    }

    /// Flush data currently in the buffer to the flash module for
    /// persistant storage. This performes a complete rewrite of
    /// the flash sector.
    #[inline]
    pub fn flush(&mut self) {
        while self.flash.sr.read().bsy().bit_is_set() {}

        unsafe {
            self.flash
                .cr
                .write(|w| w.snb().bits(0b1011).strt().set_bit())
        };

        while self.flash.sr.read().bsy().bit_is_set() {}

        for (index, byte) in self.buffer.iter().enumerate() {
            self.flash_sector[index] = *byte;
        }
    }

    #[inline]
    pub fn get(&self, index: usize) -> u8 {
        self.buffer[index]
    }

    #[inline]
    pub fn set(&mut self, index: usize, value: u8) {
        self.buffer[index] = value;
    }

    /// Return ownership of the FLASH peripheral.
    ///
    /// Data in the buffer must be flushed before freeing Flash
    pub fn free(self) -> FLASH {
        self.flash
    }
}
