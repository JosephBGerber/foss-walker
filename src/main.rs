#![feature(alloc_error_handler)]
#![no_std]
#![no_main]

#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

extern crate alloc;
use alloc::boxed::Box;
use alloc_cortex_m::CortexMHeap;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

use panic_semihosting;
use cortex_m_semihosting::{debug, hprintln};

use cortex_m;
use stm32f4xx_hal as hal;
use embedded_hal;
use rtfm::app;

use crate::hal::{prelude::*, stm32, spi, gpio::{self, ExtiPin}};
use stm32::Interrupt;

use lis3dsh::{LIS3DSH, I16x3};
use accelerometer::Accelerometer;
type Alternate5 = gpio::Alternate<gpio::AF5>;
type SPI = spi::Spi<stm32::SPI1, (gpio::gpioa::PA5<Alternate5>, gpio::gpioa::PA6<Alternate5>, gpio::gpioa::PA7<Alternate5>)>;
type CS =  gpio::gpioe::PE3<gpio::Output<gpio::PushPull>>;

mod flash;

#[app(device = stm32f4xx_hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        #[init(0)]
        presses: usize,
        accelerometer: LIS3DSH<SPI, CS>,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let mut core: cortex_m::Peripherals = cx.core;
        let device: stm32::Peripherals = cx.device;


        let start = cortex_m_rt::heap_start() as usize;
        let size = 1024 * 28;
        unsafe { ALLOCATOR.init(start, size) }

        unsafe { cortex_m::peripheral::NVIC::unmask(stm32::Interrupt::EXTI0); }


        let gpioa = device.GPIOA.split();
        let mut user = gpioa.pa0.into_floating_input();
        let sck = gpioa.pa5.into_alternate_af5();
        let miso = gpioa.pa6.into_alternate_af5();
        let mosi = gpioa.pa7.into_alternate_af5();

        let gpioe = device.GPIOE.split();
        let mut pe0 = gpioe.pe0.into_floating_input();
        let cs = gpioe.pe3.into_push_pull_output();

        let rcc = device.RCC.constrain();
        let clocks = rcc.cfgr.freeze();

        let mut syscfg = device.SYSCFG;
        let mut exti = device.EXTI;
        // pe0.make_interrupt_source(&mut syscfg);
        // pe0.enable_interrupt(&mut exti);
        // pe0.trigger_on_edge(&mut exti, hal::gpio::Edge::RISING);
        // rtfm::pend(stm32::Interrupt::EXT0);


        // user.make_interrupt_source(&mut syscfg);
        // user.enable_interrupt(&mut exti);
        // user.trigger_on_edge(&mut exti, hal::gpio::Edge::RISING);
        
        
        let pins = (sck, miso, mosi);
        let spi = hal::spi::Spi::spi1(device.SPI1, pins, embedded_hal::spi::MODE_3, 4_000_000.hz(), clocks);

        let mut accelerometer= LIS3DSH::new(spi, cs).unwrap();
        accelerometer.enable_dr_interrupt().unwrap();
        let data = accelerometer.acceleration().unwrap();
        accelerometer.calibrate_acceleration(data).unwrap();

        hprintln!("INIT").unwrap();

        init::LateResources{accelerometer}
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        hprintln!("IDLE").unwrap();
        
        loop {
            rtfm::pend(stm32::Interrupt::EXTI0);
        }
    }

    #[task(binds = EXTI0, resources = [presses, accelerometer])]
    fn exti0(cx: exti0::Context) {
        // let accelerometer = cx.resources.accelerometer;

        // let acceleration = accelerometer.acceleration().unwrap();

        // if acceleration.x > 30000 {
        //     hprintln!("Whoa slow down skippy.").unwrap();
        // }

        let presses: &mut usize = cx.resources.presses;
        *presses += 1;
        hprintln!("User pressed {} time(s)!",*presses).unwrap();
    }
};


#[alloc_error_handler]
pub fn rust_oom(_: core::alloc::Layout) -> ! {
    panic!("OUT OF MEMORY");
}