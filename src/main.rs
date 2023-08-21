#![no_std]
#![no_main]

mod layout;

use panic_halt as _;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[rtic::app(device = rp2040_hal::pac, peripherals = true, dispatchers = [PIO0_IRQ_0, PIO0_IRQ_1, PIO1_IRQ_0])]
mod app {
    use crate::layout::{LEFT_LAYER, RIGHT_LAYER};
    use cortex_m::delay::Delay;
    use cortex_m::prelude::{
        _embedded_hal_watchdog_Watchdog, _embedded_hal_watchdog_WatchdogEnable,
    };
    use embedded_hal::digital::v2::InputPin;
    use embedded_time::duration::Extensions;
    use keyberon::debounce::Debouncer;
    use keyberon::key_code;
    use keyberon::layout::{Event, Layout};
    use keyberon::matrix::Matrix;
    use rp2040_hal::clocks::init_clocks_and_plls;
    use rp2040_hal::gpio::{DynPin, Pins};
    use rp2040_hal::pac::UART0;
    use rp2040_hal::timer::{Alarm, Alarm0};
    use rp2040_hal::usb::UsbBus;
    use rp2040_hal::{Clock, Sio, Timer, Watchdog};
    use usb_device::class_prelude::UsbBusAllocator;
    use usb_device::class_prelude::UsbClass;
    use usb_device::prelude::UsbDeviceState;

    // TODO: cleanup some shared resources into local
    #[shared]
    struct Shared {
        // USB Related functions
        usb_dev: usb_device::device::UsbDevice<'static, UsbBus>,
        // TODO: add leds
        usb_class: keyberon::Class<'static, UsbBus, ()>,
        // TODO: add usb connection somehow

        // Utils
        #[lock_free]
        delay: Delay,
        timer: Timer,
        alarm: Alarm0,
        #[lock_free]
        watchdog: Watchdog,

        // KB
        layout: Layout<9, 6, 1, ()>,
        #[lock_free]
        matrix: Matrix<DynPin, DynPin, 9, 6>,
        #[lock_free]
        debouncer: Debouncer<[[bool; 9]; 6]>,
        left_side: bool,
        uart: UART0,
    }

    static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;
    const TIMER: u32 = 500;

    #[local]
    struct Local {}

    #[init]
    fn init(c: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut resets = c.device.RESETS;
        let mut watchdog = Watchdog::new(c.device.WATCHDOG);
        watchdog.pause_on_debug(false);

        let clocks = init_clocks_and_plls(
            12_000_000u32,
            c.device.XOSC,
            c.device.CLOCKS,
            c.device.PLL_SYS,
            c.device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        let sio = Sio::new(c.device.SIO);
        let pins = Pins::new(
            c.device.IO_BANK0,
            c.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );

        let side = pins.gpio4.into_floating_input();
        let left_side = side.is_high().unwrap();

        let mut timer = Timer::new(c.device.TIMER, &mut resets);
        let delay = Delay::new(c.core.SYST, clocks.system_clock.freq().0);
        let mut alarm = timer.alarm_0().unwrap();
        let _ = alarm.schedule(TIMER.microseconds());
        alarm.enable_interrupt();

        // Enable UART0
        resets.reset.modify(|_, w| w.uart0().clear_bit());
        // Wait for clear
        while resets.reset_done.read().uart0().bit_is_clear() {}
        let uart = c.device.UART0;
        // Baudrate is configured as integer / fraction
        // Integer = 67
        uart.uartibrd.write(|w| unsafe { w.bits(0b0100_0011) });
        // Decimal = 52
        uart.uartfbrd.write(|w| unsafe { w.bits(0b0011_0100) });
        uart.uartlcr_h.write(|w| unsafe { w.bits(0b0110_0000) });
        uart.uartcr.write(|w| unsafe { w.bits(0b11_0000_0001) });
        uart.uartimsc.write(|w| w.rxim().set_bit());

        let usb_bus = UsbBusAllocator::new(UsbBus::new(
            c.device.USBCTRL_REGS,
            c.device.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut resets,
        ));

        unsafe {
            USB_BUS = Some(usb_bus);
        }

        let usb_class = keyberon::new_class(unsafe { USB_BUS.as_ref().unwrap() }, ());
        let usb_dev = keyberon::new_device(unsafe { USB_BUS.as_ref().unwrap() });

        watchdog.start(10_000.microseconds());

        let rows = if left_side {
            [
                pins.gpio26.into_push_pull_output().into(),
                pins.gpio25.into_push_pull_output().into(),
                pins.gpio19.into_push_pull_output().into(),
                pins.gpio24.into_push_pull_output().into(),
                pins.gpio17.into_push_pull_output().into(),
                pins.gpio16.into_push_pull_output().into(),
            ]
        } else {
            [
                pins.gpio26.into_push_pull_output().into(),
                pins.gpio16.into_push_pull_output().into(),
                pins.gpio19.into_push_pull_output().into(),
                pins.gpio17.into_push_pull_output().into(),
                pins.gpio9.into_push_pull_output().into(),
                pins.gpio8.into_push_pull_output().into(),
            ]
        };

        (
            Shared {
                usb_dev,
                usb_class,
                delay,
                timer,
                alarm,
                watchdog,
                layout: if left_side {
                    Layout::new(&LEFT_LAYER)
                } else {
                    Layout::new(&RIGHT_LAYER)
                },
                // COL2ROW
                matrix: Matrix::new(
                    [
                        pins.gpio29.into_pull_up_input().into(),
                        pins.gpio28.into_pull_up_input().into(),
                        pins.gpio27.into_pull_up_input().into(),
                        pins.gpio7.into_pull_up_input().into(),
                        pins.gpio2.into_pull_up_input().into(),
                        pins.gpio3.into_pull_up_input().into(),
                        pins.gpio11.into_pull_up_input().into(),
                        pins.gpio12.into_pull_up_input().into(),
                        pins.gpio13.into_pull_up_input().into(),
                    ],
                    rows,
                )
                .unwrap(),
                debouncer: Debouncer::new([[false; 9]; 6], [[false; 9]; 6], 10),
                left_side,
                uart,
            },
            Local {},
            init::Monotonics(),
        )
    }

    // Handles transmission
    #[task(binds = USBCTRL_IRQ, priority = 3, shared = [usb_dev, usb_class])]
    fn usb_rx(c: usb_rx::Context) {
        let usb = c.shared.usb_dev;
        let kb = c.shared.usb_class;
        (usb, kb).lock(|usb, kb| {
            if usb.poll(&mut [kb]) {
                kb.poll();
            }
        });
    }

    #[task(priority = 2, capacity = 8, shared = [usb_dev, usb_class, layout])]
    fn handle_event(mut c: handle_event::Context, event: Option<Event>) {
        let mut layout = c.shared.layout;
        match event {
            None => {
                layout.lock(|l| l.tick());
                // if let CustomEvent::Press(event) = layout.lock(|l| l.tick()) {
                //     match event {
                //         kb_layout::CustomActions::Underglow => {
                //             handle_underglow::spawn().unwrap();
                //         }
                //         kb_layout::CustomActions::Bootloader => {
                //             rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
                //         }
                //         kb_layout::CustomActions::Display => {
                //             handle_display::spawn().unwrap();
                //         }
                //     };
                // }
            }
            Some(e) => {
                layout.lock(|l| l.event(e));
                return;
            }
        }

        let report: key_code::KbHidReport = layout.lock(|l| l.keycodes().collect());
        if !c
            .shared
            .usb_class
            .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
        {
            return;
        }
        if c.shared.usb_dev.lock(|d| d.state()) != UsbDeviceState::Configured {
            return;
        }
        while let Ok(0) = c.shared.usb_class.lock(|k| k.write(report.as_bytes())) {}
    }

    #[task(binds = TIMER_IRQ_0, priority = 1, shared = [matrix, debouncer, delay, timer, alarm, watchdog, usb_dev, usb_class])]
    fn scan_timer_irq(mut c: scan_timer_irq::Context) {
        let mut alarm = c.shared.alarm;

        alarm.lock(|a| {
            a.clear_interrupt();
            let _ = a.schedule(TIMER.microseconds());
        });

        c.shared.watchdog.feed();
        let keys_pressed = c
            .shared
            .matrix
            .get_with_delay(|| c.shared.delay.delay_us(5))
            .unwrap();
        // let keys_pressed = c.shared.matrix.get().unwrap();
        let deb_events = c.shared.debouncer.events(keys_pressed);

        for event in deb_events {
            handle_event::spawn(Some(event)).unwrap();
        }

        handle_event::spawn(None).unwrap();
    }
}
