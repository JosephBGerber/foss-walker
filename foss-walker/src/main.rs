#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![allow(non_snake_case)]

extern crate alloc;
extern crate panic_semihosting;

use core::alloc::Layout;

use alloc_cortex_m::CortexMHeap;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::FullDuplex;
use nb::block;

use stm32f4xx_hal::{prelude::*, stm32, spi, time, timer};
use spi::{Mode, Polarity::IdleLow, Phase::CaptureOnFirstTransition};
use time::Hertz;
use timer::Timer;

use engine::{Model, Msg};
use engine::display::{OAM, HEIGHT, WIDTH};
use core::convert::Infallible;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[entry]
fn main() -> ! {
    let core = cortex_m::peripheral::Peripherals::take().unwrap();
    let device = stm32::Peripherals::take().unwrap();

    let heap_start = cortex_m_rt::heap_start() as usize;
    let heap_size = 28672; // in bytes
    unsafe { ALLOCATOR.init(heap_start, heap_size) }

    // Enable clock for SYSCFG
    unsafe {
        &(*stm32::RCC::ptr()).apb2enr.modify(|_, w| w.syscfgen().set_bit());
    }

    let rcc = device.RCC.constrain();
    let clocks = rcc.cfgr
        .use_hse(25.mhz())
        .sysclk(48.mhz())
        .require_pll48clk()
        .freeze();

    let mut timer = Timer::syst(core.SYST, 30.hz(), clocks);

    let gpioa = device.GPIOA.split();
    let input = gpioa.pa3.into_pull_down_input();
    let mut cs = gpioa.pa4.into_push_pull_output();
    let sck = gpioa.pa5.into_alternate_af5();
    let miso = gpioa.pa6.into_alternate_af5();
    let mosi = gpioa.pa7.into_alternate_af5();

    let mut spi = spi::Spi::spi1(
        device.SPI1,
        (sck, miso, mosi),
        Mode { polarity: IdleLow, phase: CaptureOnFirstTransition },
        Hertz(1_000_000_u32),
        clocks,
    );

    // Enable interrupts
    // stm32::NVIC::unpend(interrupt::EXTI0);
    // unsafe { stm32::NVIC::unmask(interrupt::EXTI0) }

    let mut model = Model::new();
    let mut last_oam = model.view();
    let mut last_input = false;

    draw(&last_oam, &mut spi, &mut cs).unwrap();

    loop {
        if input.is_high().unwrap() && !last_input {
            model.update(Msg::Pressed);
            last_input = true;
        }

        if input.is_low().unwrap() && last_input {
            last_input = false;
        }

        model.update(Msg::Tick);

        let oam = model.view();
        if last_oam != oam {
            if let Err(e) = draw(&oam, &mut spi, &mut cs) {
                hprintln!("{:?}", e).unwrap();
            }
            last_oam = oam;
        }

        block!(timer.wait()).unwrap();
    }

    // if !usb_dev.poll(&mut [&mut serial]) {
    //     continue;
    // }
    //
    // let mut buf = [0u8; 64];
    //
    // match serial.read(&mut buf) {
    //     Ok(count) if count > 0 => {
    //         let mut write_offset = 0;
    //         while write_offset < count {
    //             match serial.write(&buf[write_offset..count]) {
    //                 Ok(len) if len > 0 => {
    //                     write_offset += len;
    //                 }
    //                 _ => {}
    //             }
    //         }
    //     }
    //     _ => {}
    // }
}


fn draw<SPI: FullDuplex<u8>, PIN: OutputPin<Error = Infallible>>(oam: &OAM, spi: &mut SPI, cs: &mut PIN) -> Result<(), SPI::Error> {
    // TODO: Verify that this implementation of draw is bug free

    cs.set_high().unwrap();

    for row in 0..HEIGHT {
        block!(spi.send(0x80))?;
        block!(spi.read())?;
        block!(spi.send(reverse(row as u8)))?;
        block!(spi.read())?;

        for col in (0..WIDTH).step_by(8) {
            let mut byte: u8 = 0;

            for object in &oam.objects {
                let x: isize = col as isize - object.x as isize;
                let y: isize = row as isize - object.y as isize;


                if y >= 0 && y < object.height as isize {
                    if x >= 0 && x < object.width as isize {
                        let y = y as usize;
                        let x = x as usize;
                        let left = *object.sprite.get((x / 8) + (y * (object.width as usize / 8))).unwrap_or(&0);
                        byte |= left << x % 8;
                    }
                    if (x + 7) >= 0 && (x + 7) < object.width as isize {
                        if x < 0 {
                            let y = y as usize;
                            let x = (x + 7) as usize;
                            let right = *object.sprite.get((x / 8) + (y * (object.width as usize / 8))).unwrap_or(&0);
                            byte |= right >> (7 - x % 8);
                        } else {
                            let y = y as usize;
                            let x = x as usize;
                            let right = *object.sprite.get((x / 8) + (y * (object.width as usize / 8)) + 1).unwrap_or(&0);
                            byte |= right >> (7 - x % 8);
                        }
                    }
                }
            }
            block!(spi.send(!byte))?;
            block!(spi.read())?;
        }
    }

    cs.set_low().unwrap();

// window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();

    Ok(())
}

/// Reverse the bits in a byte
///
/// ```
/// assert_eq!(reverse(0b10100000), 0b00000101);
/// assert_eq!(reverse(0b11001001), 0b10010011);
/// ```
#[allow(dead_code)]
fn reverse(byte: u8) -> u8 {
    let mut byte = byte;
    byte = (byte & 0xF0) >> 4 | (byte & 0x0F) << 4;
    byte = (byte & 0xCC) >> 2 | (byte & 0x33) << 2;
    byte = (byte & 0xAA) >> 1 | (byte & 0x55) << 1;
    return byte;
}

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    loop {}
}
