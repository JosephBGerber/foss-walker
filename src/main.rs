#![feature(alloc_error_handler)]
#![no_std]
#![no_main]

#![allow(dead_code, unused_variables, unused_imports)]

extern crate alloc;
use alloc_cortex_m::CortexMHeap;
use alloc::boxed::Box;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

use panic_semihosting;
use cortex_m_semihosting::{debug, hprintln};

use cortex_m;
use stm32f4xx_hal as hal;
use embedded_hal;
use rtfm::app;

use crate::hal::{prelude::*, stm32, spi, gpio::ExtiPin};

use accelerometer::Accelerometer;

mod flash;
mod lis3dsh;

use lis3dsh::{LIS3DSH, I16x3};

#[app(device = stm32f4xx_hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        #[init(0)]
        interrupts: usize,
        accelerometer: Box<dyn Accelerometer<I16x3, Error = lis3dsh::Error<spi::Error, !>>>,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let core: cortex_m::Peripherals = cx.core;

        let device: stm32::Peripherals = cx.device;
        
        let start = cortex_m_rt::heap_start() as usize;
        let size = 1024;
        unsafe { ALLOCATOR.init(start, size) }

        let mut exti1 = device.EXTI;

        let gpioe = device.GPIOE.split();
        let mut pe0 = gpioe.pe0.into_floating_input();
        let cs = gpioe.pe3.into_push_pull_output();

        pe0.trigger_on_edge(&mut exti1, hal::gpio::Edge::RISING);

        let gpioa = device.GPIOA.split();
        let sck = gpioa.pa5.into_alternate_af5();
        let miso = gpioa.pa6.into_alternate_af5();
        let mosi = gpioa.pa7.into_alternate_af5();
        let pins = (sck, miso, mosi);

        let rcc = device.RCC.constrain();
        let clocks = rcc.cfgr.freeze();

        let spi = hal::spi::Spi::spi1(device.SPI1, pins, embedded_hal::spi::MODE_3, 4_000_000.hz(), clocks);

        let mut accelerometer= LIS3DSH::new(spi, cs).unwrap();
        accelerometer.enable_dr_interrupt().unwrap();
        let acceleration = accelerometer.acceleration().unwrap();
        accelerometer.calibrate_acceleration(acceleration).unwrap();

        let accelerometer = Box::from(accelerometer);

        hprintln!("INIT").unwrap();

        init::LateResources{accelerometer}
    }

    #[task(binds = EXTI1, resources = [interrupts])]
    fn EXTI1(cx: EXTI1::Context) {
        let interrupts: &mut usize = cx.resources.interrupts;

        *interrupts += 1;

        hprintln!(
            "EXTI1 called {} time{}",
            *interrupts,
            if *interrupts > 1 { "s" } else { "" }
        )
        .unwrap();
    }
};


#[alloc_error_handler]
pub fn rust_oom(_: core::alloc::Layout) -> ! {
    panic!("OUT OF MEMORY");
}