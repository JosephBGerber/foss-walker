use stm32f4xx_hal::stm32::FLASH;
use flash_eeprom::EEPROM;
use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};

pub struct STM32EEPROM(FLASH);

impl STM32EEPROM {
    pub(crate) fn new(flash: FLASH) -> STM32EEPROM {
        unsafe {
            while flash.sr.read().bsy().bit() {}
            flash.keyr.write(|w| w.bits(0x45670123));
            while flash.sr.read().bsy().bit() {}
            flash.keyr.write(|w| w.bits(0xCDEF89AB));
            flash.cr.write(|w| w.psize().bits(0b10));
        }
        STM32EEPROM(flash)
    }
}

impl EEPROM<4> for STM32EEPROM {
    unsafe fn get_pages(&self) -> [&[usize]; 4] {
        [
            &*slice_from_raw_parts(0x0804_0000 as *const usize, 32767),
            &*slice_from_raw_parts(0x0806_0000 as *const usize, 32767),
            &*slice_from_raw_parts(0x0808_0000 as *const usize, 32767),
            &*slice_from_raw_parts(0x080A_0000 as *const usize, 32767)
        ]
    }

    unsafe fn get_pages_mut(&mut self) -> [&mut [usize]; 4] {
        [
            &mut *slice_from_raw_parts_mut(0x0804_0000 as *mut usize, 32767),
            &mut *slice_from_raw_parts_mut(0x0806_0000 as *mut usize, 32767),
            &mut *slice_from_raw_parts_mut(0x0808_0000 as *mut usize, 32767),
            &mut *slice_from_raw_parts_mut(0x080A_0000 as *mut usize, 32767)
        ]
    }

    unsafe fn reset_page(&mut self, index: usize) {
        // Check that no flash memory operation is ongoing
        while self.0.sr.read().bsy().bit() {}

        let sector = match index {
            0 => 6,
            1 => 7,
            2 => 8,
            3 => 9,
            _ => panic!("reset_page: invalid page index")
        };

        // self.0.cr.modify(|_, w| { w.ser().set_bit().snb().bits(sector) });
        // self.0.cr.modify(|_, w| { w.strt().set_bit() });

        self.0.cr.write(|w| w.psize().bits(0b10).ser().set_bit().snb().bits(sector).strt().set_bit());


        //Wait for the BSY flag to be cleared
        while self.0.sr.read().bsy().bit() {}
    }
}