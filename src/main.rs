#![no_std]
#![no_main]
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use cortex_m_semihosting::{debug, hprintln};
use panic_semihosting;

use cortex_m;
use embedded_hal;
use rtfm::app;
use stm32f4xx_hal as hal;

use crate::hal::{gpio, interrupt, prelude::*, spi, stm32, timer};
use gpio::{ExtiPin, Input, Output};
use stm32::EXTI;
use timer::{Event, Timer};

use lis3dsh::LIS3DSH;
use accelerometer::Accelerometer;

type Alternate = gpio::Alternate<gpio::AF5>;
type SPI = spi::Spi<
    stm32::SPI1,
    (
        gpio::gpioa::PA5<Alternate>,
        gpio::gpioa::PA6<Alternate>,
        gpio::gpioa::PA7<Alternate>,
    ),
>;
type CS = gpio::gpioe::PE3<Output<gpio::PushPull>>;

#[app(device = stm32f4xx_hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        exti: EXTI,
        int1: gpio::gpioe::PE0<Input<gpio::PullDown>>,
        green: gpio::gpiod::PD12<Output<gpio::PushPull>>,
        blue: gpio::gpiod::PD15<Output<gpio::PushPull>>,
        accelerometer: LIS3DSH<SPI, CS>,
        timer: Timer<stm32::TIM2>,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let mut core: cortex_m::Peripherals = cx.core;
        let device: stm32::Peripherals = cx.device;

        // Unsafe usage of RCC_APB2EN register to enable SYSCFGEN clock
        // Required to configure EXTI0 register
        let rcc_register_block = unsafe { &(*stm32::RCC::ptr()) };
        rcc_register_block.apb2enr.modify(|_, w| {
            w.syscfgen().set_bit()
        });

        let rcc = device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(26_000_000.hz()).freeze();

        let gpioa = device.GPIOA.split();
        let sck = gpioa.pa5.into_alternate_af5();
        let miso = gpioa.pa6.into_alternate_af5();
        let mosi = gpioa.pa7.into_alternate_af5();

        let pins = (sck, miso, mosi);

        let spi = spi::Spi::spi1(
            device.SPI1,
            pins,
            embedded_hal::spi::MODE_3,
            5_000_000.hz(),
            clocks,
        );

        let gpiod = device.GPIOD.split();
        let green = gpiod.pd12.into_push_pull_output();
        let blue = gpiod.pd15.into_push_pull_output();

        let gpioe = device.GPIOE.split();
        let mut int1 = gpioe.pe0.into_pull_down_input();
        let cs = gpioe.pe3.into_push_pull_output();

        let mut accelerometer = LIS3DSH::new(spi, cs).unwrap();
        accelerometer.enable_dr_interrupt().unwrap();

        let data = accelerometer.acceleration().unwrap();

        hprintln!("{:?}", data).unwrap();

        let mut syscfg = device.SYSCFG;
        let mut exti = device.EXTI;
        let tim2 = device.TIM2;

        let mut tim2 = Timer::tim2(tim2, 1.hz(), clocks);
        tim2.listen(Event::TimeOut);

        int1.make_interrupt_source(&mut syscfg);
        int1.enable_interrupt(&mut exti);
        int1.trigger_on_edge(&mut exti, gpio::Edge::RISING);

        init::LateResources {
            exti,
            int1,
            green,
            blue,
            accelerometer,
            timer: tim2,
        }
    }

    #[task(binds = EXTI0, resources = [exti, accelerometer, int1, blue])]
    fn exti0(cx: exti0::Context) {
        let mut exti = cx.resources.exti;
        let mut accelerometer = cx.resources.accelerometer;
        let mut int1 = cx.resources.int1;
        let mut blue = cx.resources.blue;

        int1.clear_interrupt_pending_bit(&mut exti);

        hprintln!("EXTI0").unwrap();

        let data = accelerometer.acceleration().unwrap();

        if data.x.abs() > 100 {
            blue.set_high().unwrap();
        } else {
            blue.set_low().unwrap();
        }


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
