use stm32f4xx_hal::stm32::FLASH;
use flash_eeprom::EEPROM;
use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};

pub struct STM32EEPROM(FLASH);

impl STM32EEPROM {
    pub(crate) fn new(flash: FLASH) -> STM32EEPROM {
        unsafe {
            while flash.sr.read().bsy().bit_is_set() {}
            flash.keyr.write(|w| w.bits(0x45670123));
            flash.keyr.write(|w| w.bits(0xCDEF89AB));
            flash.cr.modify(|_, w| w.psize().bits(0b10).pg().set_bit());
        }

        STM32EEPROM(flash)
    }
}

impl EEPROM<4> for STM32EEPROM {
    unsafe fn get_pages(&self) -> [&[usize]; 3] {
        [
            &*slice_from_raw_parts(0x0802_0000 as *const usize, 32767),
            &*slice_from_raw_parts(0x0804_0000 as *const usize, 32767),
            &*slice_from_raw_parts(0x0806_0000 as *const usize, 32767),
        ]
    }

    unsafe fn get_pages_mut(&mut self) -> [&mut [usize]; 3] {
        [
            &mut *slice_from_raw_parts_mut(0x0802_0000 as *mut usize, 32767),
            &mut *slice_from_raw_parts_mut(0x0804_0000 as *mut usize, 32767),
            &mut *slice_from_raw_parts_mut(0x0806_0000 as *mut usize, 32767),
        ]
    }

    unsafe fn reset_page(&mut self, index: usize) {
        // Check that no flash memory operation is ongoing
        while self.0.sr.read().bsy().bit_is_set() {}

        let sector = match index {
            0 => 5,
            1 => 6,
            2 => 7,
            _ => panic!("reset_page: invalid page index")
        };

        self.0.cr.modify(|_, w| { w.ser().set_bit().snb().bits(sector) });
        self.0.cr.modify(|_, w| { w.strt().set_bit() });

        //Wait for the BSY flag to be cleared
        while self.0.sr.read().bsy().bit_is_set() {}
    }
}