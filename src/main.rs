#![no_std]
#![no_main]

// pick a panicking behavior
// extern crate panic_halt; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// extern crate panic_abort; // requires nightly
// extern crate panic_itm; // logs messages over ITM; requires ITM support
extern crate panic_semihosting;

use cortex_m;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;

use crate::hal::{prelude::*, stm32};
use stm32f4xx_hal as hal;

#[entry]
fn main() -> ! {
    let cortex_p = cortex_m::Peripherals::take().unwrap();
    let p = stm32::Peripherals::take().unwrap();

    unsafe {
        p.FLASH.keyr.write(|w| w.bits(0x4567_0123));
        p.FLASH.keyr.write(|w| w.bits(0xCDEF_89AB));
    };

    let gpioa = p.GPIOA.split();
    let mut led = gpioa.pa0.into_push_pull_output();

    let rcc = p.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

    let mut delay = hal::delay::Delay::new(cortex_p.SYST, clocks);

    loop {
        led.set_high().unwrap();
        hprintln!("ON").unwrap();
        delay.delay_ms(1000_u32);
        led.set_low().unwrap();
        hprintln!("OFF").unwrap();
        delay.delay_ms(1000_u32);
    }
}
