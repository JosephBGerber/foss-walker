#![no_std]
#![no_main]
#![feature(lang_items)]
#![allow(dead_code, unused_variables, unused_imports, unused_mut, clippy::missing_safety_doc)]

extern crate alloc;
use alloc::boxed::Box;

use cortex_m_semihosting::{debug, hprintln};
use panic_semihosting;

use cortex_m;
use embedded_hal;
use rtfm::app;
use stm32f4xx_hal as hal;

use alloc_cortex_m::CortexMHeap;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

use crate::hal::{gpio, interrupt, prelude::*, spi, stm32, timer};
use gpio::{ExtiPin, Input, Output};
use stm32::EXTI;
use timer::{Event, Timer};

mod graphics;
use graphics::Display;

type Alternate = gpio::Alternate<gpio::AF5>;
type SPI = spi::Spi<
    stm32::SPI1,
    (
        gpio::gpioa::PA5<Alternate>,
        gpio::gpioa::PA6<Alternate>,
        gpio::gpioa::PA7<Alternate>,
    ),
>;

// # Safety - only use on the stm32f407 mcu
#[app(device = stm32f4xx_hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        exti: EXTI,
        int1: gpio::gpioe::PE0<Input<gpio::PullDown>>,
        timer: Timer<stm32::TIM2>,

        user: gpio::gpioa::PA0<Input<gpio::PullDown>>,
        green: gpio::gpiod::PD12<Output<gpio::PushPull>>,
        blue: gpio::gpiod::PD15<Output<gpio::PushPull>>,

        spi: SPI,
        display: Display,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let mut core: cortex_m::Peripherals = cx.core;
        let device: stm32::Peripherals = cx.device;

        // Initialize the heap
        let start = cortex_m_rt::heap_start() as usize;
        let size = 2048;
        unsafe { ALLOCATOR.init(start, size) };

        // Unsafe usage of RCC_APB2EN register to enable SYSCFGEN clock
        // Required to configure EXTI0 register
        let rcc = unsafe { &(*stm32::RCC::ptr()) };
        rcc.apb2enr.modify(|_, w| w.syscfgen().set_bit());

        // Take the rcc peripheral
        let rcc = device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(28_000_000.hz()).freeze();

        let gpioa = device.GPIOA.split();
        let user = gpioa.pa0.into_pull_down_input();
        let sck = gpioa.pa5.into_alternate_af5();
        let miso = gpioa.pa6.into_alternate_af5();
        let mosi = gpioa.pa7.into_alternate_af5();

        let pins = (sck, miso, mosi);
        let spi = spi::Spi::spi1(
            device.SPI1,
            pins,
            embedded_hal::spi::MODE_3,
            500_000.hz(),
            clocks,
        );

        let cs = Box::from(gpioa.pa10.into_push_pull_output());
        let en = Box::from(gpioa.pa11.into_push_pull_output());
        let display = Display::new(cs, en);

        let gpiod = device.GPIOD.split();
        let green = gpiod.pd12.into_push_pull_output();
        let blue = gpiod.pd15.into_push_pull_output();

        let gpioe = device.GPIOE.split();
        let mut int1 = gpioe.pe0.into_pull_down_input();
        let cs = gpioe.pe3.into_push_pull_output();

        let mut syscfg = device.SYSCFG;
        let mut exti = device.EXTI;
        let tim2 = device.TIM2;

        int1.make_interrupt_source(&mut syscfg);
        int1.enable_interrupt(&mut exti);
        int1.trigger_on_edge(&mut exti, gpio::Edge::RISING);

        let mut tim2 = Timer::tim2(tim2, 30.hz(), clocks);
        tim2.listen(Event::TimeOut);

        init::LateResources {
            exti,
            int1,
            timer: tim2,
            user,
            green,
            blue,
            spi,
            display,
        }
    }

    #[task(binds = EXTI0, resources = [exti, int1, blue])]
    fn exti0(cx: exti0::Context) {
        let mut exti = cx.resources.exti;
        let mut int1 = cx.resources.int1;
        //let mut blue = cx.resources.blue;

        int1.clear_interrupt_pending_bit(&mut exti);
    }

    #[task(binds = TIM2, resources = [timer, green])]
    fn tim2(cx: tim2::Context) {
        static mut STATE: bool = true;

        let mut timer = cx.resources.timer;
        let mut green = cx.resources.green;

        timer.clear_interrupt(Event::TimeOut);

        if *STATE {
            green.set_high().unwrap();
            *STATE = false;
        } else {
            green.set_low().unwrap();
            *STATE = true;
        }
    }
};

#[lang = "oom"]
#[no_mangle]
pub fn rust_oom(_: core::alloc::Layout) -> ! {
    panic!("OUT OF MEMORY")
}
