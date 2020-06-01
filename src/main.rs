#![no_std]
#![no_main]
#![allow(non_snake_case)]

extern crate panic_semihosting;

use core::cell::RefCell;
use cortex_m::{self, interrupt::Mutex};
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use embedded_hal::spi::Polarity::IdleLow;
use embedded_hal::spi::Phase::CaptureOnFirstTransition;
use flash_eeprom::EEPROM;

use stm32f4xx_hal::{prelude::*, gpio, interrupt, otg_fs, stm32, spi, time};
use gpio::ExtiPin;
// use otg_fs::{UsbBus, USB};
use spi::*;
use time::Hertz;

// use usb_device::prelude::*;

mod eeprom;

use eeprom::STM32EEPROM;


static USER_INPUT_MUTEX: Mutex<RefCell<Option<gpio::gpioa::PA0<gpio::Input<gpio::Floating>>>>> =
    Mutex::new(RefCell::new(None));
static GREEN_LED_MUTEX: Mutex<RefCell<Option<gpio::gpiod::PD12<gpio::Output<gpio::PushPull>>>>> =
    Mutex::new(RefCell::new(None));
static FLASH_MUTEX: Mutex<RefCell<Option<STM32EEPROM>>> = Mutex::new(RefCell::new(None));

// static mut EP_MEMORY: [u32; 1024] = [0; 1024];

#[entry]
fn main() -> ! {
    let _core = cortex_m::peripheral::Peripherals::take().unwrap();
    let device = stm32::Peripherals::take().unwrap();

    let rcc_register_block = unsafe { &(*stm32::RCC::ptr()) };

    // Enable clock for SYSCFG
    rcc_register_block
        .apb2enr
        .modify(|_, w| w.syscfgen().set_bit());

    let rcc = device.RCC.constrain();
    let clocks = rcc.cfgr
        .use_hse(25.mhz())
        .sysclk(48.mhz())
        .require_pll48clk()
        .freeze();

    let mut syscfg = device.SYSCFG;
    let mut exti = device.EXTI;

    // x 0x4002_3c10 - read sr

    let mut eeprom = STM32EEPROM::new(device.FLASH);

    for i in 0..4 {
        unsafe { eeprom.reset_page(i); }
    }

    let gpioa = device.GPIOA.split();
    let mut user_input = gpioa.pa0.into_floating_input();
    let sck = gpioa.pa5.into_alternate_af5();
    let miso = gpioa.pa6.into_alternate_af5();
    let mosi = gpioa.pa7.into_alternate_af5();

    let mut _spi = spi::Spi::spi1(
        device.SPI1,
        (sck, miso, mosi),
        Mode { polarity: IdleLow, phase: CaptureOnFirstTransition },
        Hertz(1_000_000_u32),
        clocks,
    );

    user_input.enable_interrupt(&mut exti);
    user_input.make_interrupt_source(&mut syscfg);
    user_input.trigger_on_edge(&mut exti, gpio::Edge::RISING_FALLING);

    let gpiod = device.GPIOD.split();
    let green_led = gpiod.pd12.into_push_pull_output();

    cortex_m::interrupt::free(|cs| {
        USER_INPUT_MUTEX.borrow(cs).replace(Some(user_input));
        GREEN_LED_MUTEX.borrow(cs).replace(Some(green_led));
        FLASH_MUTEX.borrow(cs).replace(Some(eeprom));
    });

    // let usb = USB {
    //     usb_global: device.OTG_FS_GLOBAL,
    //     usb_device: device.OTG_FS_DEVICE,
    //     usb_pwrclk: device.OTG_FS_PWRCLK,
    //     pin_dm: gpioa.pa11.into_alternate_af10(),
    //     pin_dp: gpioa.pa12.into_alternate_af10(),
    // };
    //
    // let usb_bus = UsbBus::new(usb, unsafe { &mut EP_MEMORY });
    //
    // let mut serial = usbd_serial::SerialPort::new(&usb_bus);
    //
    // let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
    //     .manufacturer("Joseph Gerber")
    //     .product("foss-walker")
    //     .serial_number("TEST")
    //     .device_class(usbd_serial::USB_CLASS_CDC)
    //     .build();

    // Enable interrupts
    stm32::NVIC::unpend(interrupt::EXTI0);
    unsafe { stm32::NVIC::unmask(interrupt::EXTI0) }

    hprintln!("Variable 1: {}", 0).unwrap();

    loop {
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
}

#[interrupt]
fn EXTI0() {
    cortex_m::interrupt::free(|cs| {
        let mut user_input = USER_INPUT_MUTEX.borrow(cs).borrow_mut();
        let mut green_led = GREEN_LED_MUTEX.borrow(cs).borrow_mut();
        let mut eeprom = FLASH_MUTEX.borrow(cs).borrow_mut();

        let user_input = user_input.as_mut().unwrap();
        let green_led = green_led.as_mut().unwrap();
        let eeprom = eeprom.as_mut().unwrap();


        user_input.clear_interrupt_pending_bit();

        if user_input.is_high().unwrap() {
            green_led.set_high().unwrap();

            let mut count = if let Some(slice) = eeprom.read(1) {
                slice[0]
            } else {
                0
            };

            hprintln!("Variable 1: {}", count).unwrap();

            count += 1;

            eeprom.write(1, &[count]);
        } else {
            green_led.set_low().unwrap();
        }
    });
}

/// Reverse the bits in a byte
///
/// ```
/// assert_eq!(reverse(0b10100000), 0b00000101);
/// assert_eq!(reverse(0b11001001), 0b10010011);
/// ```
fn reverse(byte: u8) -> u8 {
    let mut byte = byte;
    byte = (byte & 0xF0) >> 4 | (byte & 0x0F) << 4;
    byte = (byte & 0xCC) >> 2 | (byte & 0x33) << 2;
    byte = (byte & 0xAA) >> 1 | (byte & 0x55) << 1;
    return byte;
}

// #![feature(lang_items)]
// #![allow(dead_code, unused_variables, unused_imports, unused_mut, clippy::missing_safety_doc)]
//
// extern crate alloc;
// use alloc::boxed::Box;
//
// use cortex_m_semihosting::{debug, hprintln};
// use panic_semihosting;
//
// use cortex_m;
// use embedded_hal;
// use rtfm::app;
// use stm32f4xx_hal as hal;
//
// use alloc_cortex_m::CortexMHeap;
//
// #[global_allocator]
// static ALLOCATOR: CortexMHeap = CortexMHeap::empty();
//
// use crate::hal::{gpio, interrupt, prelude::*, spi, stm32, timer};
// use gpio::{ExtiPin, Input, Output};
// use stm32::EXTI;
// use timer::{Event, Timer};
//
// mod graphics;
// use graphics::Display;
//
// type Alternate = gpio::Alternate<gpio::AF5>;
// type SPI = spi::Spi<
//     stm32::SPI1,
//     (
//         gpio::gpioa::PA5<Alternate>,
//         gpio::gpioa::PA6<Alternate>,
//         gpio::gpioa::PA7<Alternate>,
//     ),
// >;
//
// // # Safety - only use on the stm32f407 mcu
// #[app(device = stm32f4xx_hal::stm32, peripherals = true)]
// const APP: () = {
//     struct Resources {
//         exti: EXTI,
//         int1: gpio::gpioe::PE0<Input<gpio::PullDown>>,
//         timer: Timer<stm32::TIM2>,
//
//         user: gpio::gpioa::PA0<Input<gpio::PullDown>>,
//         green: gpio::gpiod::PD12<Output<gpio::PushPull>>,
//         blue: gpio::gpiod::PD15<Output<gpio::PushPull>>,
//
//         spi: SPI,
//         display: Display,
//     }
//
//     #[init]
//     fn init(cx: init::Context) -> init::LateResources {
//         let mut core: cortex_m::Peripherals = cx.core;
//         let device: stm32::Peripherals = cx.device;
//
//         // Initialize the heap
//         let start = cortex_m_rt::heap_start() as usize;
//         let size = 2048;
//         unsafe { ALLOCATOR.init(start, size) };
//
//         // Unsafe usage of RCC_APB2EN register to enable SYSCFGEN clock
//         // Required to configure EXTI0 register
//         let rcc = unsafe { &(*stm32::RCC::ptr()) };
//         rcc.apb2enr.modify(|_, w| w.syscfgen().set_bit());
//
//         // Take the rcc peripheral
//         let rcc = device.RCC.constrain();
//         let clocks = rcc.cfgr.sysclk(28_000_000.hz()).freeze();
//
//         let gpioa = device.GPIOA.split();
//         let user = gpioa.pa0.into_pull_down_input();
//         let sck = gpioa.pa5.into_alternate_af5();
//         let miso = gpioa.pa6.into_alternate_af5();
//         let mosi = gpioa.pa7.into_alternate_af5();
//
//         let pins = (sck, miso, mosi);
//         let spi = spi::Spi::spi1(
//             device.SPI1,
//             pins,
//             embedded_hal::spi::MODE_3,
//             500_000.hz(),
//             clocks,
//         );
//
//         let cs = Box::from(gpioa.pa10.into_push_pull_output());
//         let en = Box::from(gpioa.pa11.into_push_pull_output());
//         let display = Display::new(cs, en);
//
//         let gpiod = device.GPIOD.split();
//         let green = gpiod.pd12.into_push_pull_output();
//         let blue = gpiod.pd15.into_push_pull_output();
//
//         let gpioe = device.GPIOE.split();
//         let mut int1 = gpioe.pe0.into_pull_down_input();
//         let cs = gpioe.pe3.into_push_pull_output();
//
//         let mut syscfg = device.SYSCFG;
//         let mut exti = device.EXTI;
//         let tim2 = device.TIM2;
//
//         int1.make_interrupt_source(&mut syscfg);
//         int1.enable_interrupt(&mut exti);
//         int1.trigger_on_edge(&mut exti, gpio::Edge::RISING);
//
//         let mut tim2 = Timer::tim2(tim2, 30.hz(), clocks);
//         tim2.listen(Event::TimeOut);
//
//         init::LateResources {
//             exti,
//             int1,
//             timer: tim2,
//             user,
//             green,
//             blue,
//             spi,
//             display,
//         }
//     }
//
//     #[task(binds = EXTI0, resources = [exti, int1, blue])]
//     fn exti0(cx: exti0::Context) {
//         let mut exti = cx.resources.exti;
//         let mut int1 = cx.resources.int1;
//         //let mut blue = cx.resources.blue;
//
//         int1.clear_interrupt_pending_bit(&mut exti);
//     }
// //
//     #[task(binds = TIM2, resources = [timer, green])]
//     fn tim2(cx: tim2::Context) {
//         static mut STATE: bool = true;
//
//         let mut timer = cx.resources.timer;
//         let mut green = cx.resources.green;
//
//         timer.clear_interrupt(Event::TimeOut);
//
//         if *STATE {
//             green.set_high().unwrap();
//             *STATE = false;
//         } else {
//             green.set_low().unwrap();
//             *STATE = true;
//         }
//     }
// };
//
// #[lang = "oom"]
// #[no_mangle]
// pub fn rust_oom(_: core::alloc::Layout) -> ! {
//     panic!("OUT OF MEMORY")
// }
