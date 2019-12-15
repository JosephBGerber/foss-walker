#![no_std]
#![no_main]
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use cortex_m_semihosting::{debug, hprintln};
use panic_semihosting;

use cortex_m;
use embedded_hal;
use rtfm::app;
use stm32f4xx_hal as hal;

use crate::hal::{delay, gpio, i2c, interrupt, prelude::*, stm32, timer};
use gpio::{ExtiPin, Input, Output};
use stm32::EXTI;
use timer::{Event, Timer};

use mpu6050::{Mpu6050, Steps};

type Alternate = gpio::Alternate<gpio::AF4>;
type I2C = i2c::I2c<stm32::I2C1, (gpio::gpiob::PB6<Alternate>, gpio::gpiob::PB7<Alternate>)>;
type CS = gpio::gpioe::PE3<Output<gpio::PushPull>>;

/// # Safety this binary must be run on the stm32f407
#[app(device = stm32f4xx_hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        exti: EXTI,
        int1: gpio::gpioe::PE0<Input<gpio::PullDown>>,
        green: gpio::gpiod::PD12<Output<gpio::PushPull>>,
        blue: gpio::gpiod::PD15<Output<gpio::PushPull>>,
        mpu: Mpu6050<I2C, delay::Delay>,
        timer: Timer<stm32::TIM2>,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let mut core: cortex_m::Peripherals = cx.core;
        let device: stm32::Peripherals = cx.device;

        // Unsafe usage of RCC_APB2EN register to enable SYSCFGEN clock
        // Required to configure EXTI0 register
        let rcc_register_block = unsafe { &(*stm32::RCC::ptr()) };
        rcc_register_block
            .apb2enr
            .modify(|_, w| w.syscfgen().set_bit());

        let syst = core.SYST;
        let rcc = device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(28_000_000.hz()).freeze();

        let gpioa = device.GPIOB.split();
        let scl = gpioa.pb6.into_alternate_af4();
        let sda = gpioa.pb7.into_alternate_af4();

        let pins = (scl, sda);

        let i2c = i2c::I2c::i2c1(device.I2C1, pins, 100.khz(), clocks);
        let delay = delay::Delay::new(syst, clocks);

        let gpiod = device.GPIOD.split();
        let green = gpiod.pd12.into_push_pull_output();
        let blue = gpiod.pd15.into_push_pull_output();

        let gpioe = device.GPIOE.split();
        let mut int1 = gpioe.pe0.into_pull_down_input();

        let mut mpu = Mpu6050::new(i2c, delay);
        mpu.init().unwrap();
        mpu.soft_calib(Steps(100)).unwrap();
        mpu.calc_variance(Steps(50)).unwrap();

        mpu.write_u8(0x38, 0x01).unwrap();

        if let Ok(data) = mpu.get_acc() {
            hprintln!("{:?}", data).unwrap();
        }

        let mut syscfg = device.SYSCFG;
        let mut exti = device.EXTI;
        let tim2 = device.TIM2;

        let mut tim2 = Timer::tim2(tim2, 5.hz(), clocks);
        tim2.listen(Event::TimeOut);

        int1.make_interrupt_source(&mut syscfg);
        int1.enable_interrupt(&mut exti);
        int1.trigger_on_edge(&mut exti, gpio::Edge::RISING);

        init::LateResources {
            exti,
            int1,
            green,
            blue,
            mpu,
            timer: tim2,
        }
    }

    #[task(binds = EXTI0, resources = [exti, mpu, int1, blue])]
    fn exti0(cx: exti0::Context) {
        let mut exti = cx.resources.exti;
        let mut mpu = cx.resources.mpu;
        let mut int1 = cx.resources.int1;
        let mut blue = cx.resources.blue;

        int1.clear_interrupt_pending_bit(&mut exti);

        hprintln!("EXTI0").unwrap();
        if let Ok(data) = mpu.get_acc() {
            if data.x > 100.0 || data.x < -100.0 {
                blue.set_high().unwrap();
            } else {
                blue.set_low().unwrap();
            }
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
