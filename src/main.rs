#![no_std]
#![no_main]

mod layout;

use panic_halt as _;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[rtic::app(device = rp2040_hal::pac, peripherals = true, dispatchers = [PIO0_IRQ_0, PIO0_IRQ_1, PIO1_IRQ_0])]
mod app {
    use crate::layout::{LAYER, LEFT_LAYER, RIGHT_LAYER};
    use cortex_m::delay::Delay;
    use cortex_m::prelude::{
        _embedded_hal_watchdog_Watchdog, _embedded_hal_watchdog_WatchdogEnable,
    };
    use embedded_hal::digital::v2::InputPin;
    use embedded_hal::serial::{Read, Write};
    use fugit::MicrosDurationU32;
    use keyberon::debounce::Debouncer;
    use keyberon::key_code;
    use keyberon::layout::{Event, Layout};
    use keyberon::matrix::Matrix;
    use rp2040_hal::clocks::init_clocks_and_plls;
    use rp2040_hal::gpio::bank0::{Gpio0, Gpio1};
    use rp2040_hal::gpio::{DynPin, FunctionUart, Pin, Pins};
    use rp2040_hal::pac::UART0;
    use rp2040_hal::timer::{Alarm, Alarm0};
    use rp2040_hal::uart::{Enabled, UartPeripheral};
    use rp2040_hal::usb::UsbBus;
    use rp2040_hal::{uart, Clock, Sio, Timer, Watchdog};
    use usb_device::class_prelude::UsbBusAllocator;
    use usb_device::class_prelude::UsbClass;
    use usb_device::prelude::UsbDeviceState;

    type Uart =
        UartPeripheral<Enabled, UART0, (Pin<Gpio0, FunctionUart>, Pin<Gpio1, FunctionUart>)>;

    #[derive(Copy, Clone)]
    pub enum ReceivedAction {
        Press,
        Release,
    }

    #[derive(Default)]
    pub struct RA {
        pub inner: Option<ReceivedAction>,
    }
    impl RA {
        pub fn update(&mut self, action: Option<ReceivedAction>) {
            self.inner = action
        }
    }

    // TODO: cleanup some shared resources into local
    #[shared]
    struct Shared {
        // USB Related functions
        usb_dev: usb_device::device::UsbDevice<'static, UsbBus>,
        // TODO: add leds
        usb_class: keyberon::Class<'static, UsbBus, ()>,

        // Utils
        #[lock_free]
        delay: Delay,
        timer: Timer,
        alarm: Alarm0,
        #[lock_free]
        watchdog: Watchdog,

        // KB
        main_layout: Layout<18, 6, 1, ()>,
        layout: Layout<9, 6, 1, ()>,
        #[lock_free]
        matrix: Matrix<DynPin, DynPin, 9, 6>,
        #[lock_free]
        debouncer: Debouncer<[[bool; 9]; 6]>,
        #[lock_free]
        left_side: bool,

        uart: Uart,
        #[lock_free]
        action: RA,
    }

    static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;
    const TIMER: u32 = 500;
    const PRESS_ID: u8 = 255;
    const RELEASE_ID: u8 = 254;
    const ERROR_ID: u8 = 253;

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

        let mut timer = Timer::new(c.device.TIMER, &mut resets);
        let delay = Delay::new(c.core.SYST, clocks.system_clock.freq().to_Hz());
        let mut alarm = timer.alarm_0().unwrap();
        let _ = alarm.schedule(MicrosDurationU32::micros(TIMER));
        alarm.enable_interrupt();

        // Enable UART0, we need to swap the pins when working with different sides
        let uart_pins = (
            // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
            pins.gpio0.into_mode::<FunctionUart>(),
            // UART RX (characters received by RP2040) on pin 2 (GPIO1)
            pins.gpio1.into_mode::<FunctionUart>(),
        );

        let mut uart = UartPeripheral::new(c.device.UART0, uart_pins, &mut resets)
            .enable(
                uart::common_configs::_9600_8_N_1,
                clocks.peripheral_clock.freq(),
            )
            .unwrap();
        uart.enable_rx_interrupt();

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
        // TODO: custom dev info
        let usb_dev = keyberon::new_device(unsafe { USB_BUS.as_ref().unwrap() });

        watchdog.start(MicrosDurationU32::micros(10_000u32));
        let left_side = side.is_high().unwrap();

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
                main_layout: Layout::new(&LAYER),
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
                action: RA::default(),
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

    #[task(priority = 2, capacity = 8, shared = [usb_dev, usb_class, main_layout])]
    fn handle_event(mut c: handle_event::Context, event: Option<Event>) {
        let mut layout = c.shared.main_layout;
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

    #[task(binds = TIMER_IRQ_0, priority = 1, shared = [matrix, debouncer, delay, timer, alarm, watchdog, usb_dev, usb_class, left_side, uart])]
    fn scan_timer_irq(mut c: scan_timer_irq::Context) {
        let mut alarm = c.shared.alarm;

        alarm.lock(|a| {
            a.clear_interrupt();
            let _ = a.schedule(MicrosDurationU32::micros(TIMER));
        });

        c.shared.watchdog.feed();
        let keys_pressed = c
            .shared
            .matrix
            .get_with_delay(|| c.shared.delay.delay_us(5))
            .unwrap();
        let events = c.shared.debouncer.events(keys_pressed);

        // TODO: try to handle duplex communication
        if *c.shared.left_side {
            // if c.shared.uart.uart_is_readable() {
            //     handle_event::spawn(Some(Event::Release(1, 1))).unwrap();
            // }

            for event in events {
                handle_event::spawn(Some(event)).unwrap();
            }

            c.shared.uart.lock(|uart| {
                if uart.uart_is_readable() {
                    handle_event::spawn(Some(Event::Press(3, 3))).unwrap();
                    handle_event::spawn(Some(Event::Release(3, 3))).unwrap();
                }
            });

            handle_event::spawn(None).unwrap();
        } else {
            for event in events {
                c.shared.uart.lock(|uart| {
                    if uart.uart_is_writable() {
                        handle_event::spawn(Some(Event::Press(3, 3))).unwrap();
                        handle_event::spawn(Some(Event::Release(3, 3))).unwrap();

                        //   1  /  0      1111111  | 10010 110
                        // Press/Release  Verifier | X     Y

                        // The verifier helps us know which byte were looking at, it should help us tackle desync
                        let (ident, key) = match event {
                            Event::Press(i, j) => (PRESS_ID, (i << 3) | j),
                            Event::Release(i, j) => (RELEASE_ID, (i << 3) | j),
                        };

                        uart.write_raw(&[ident, key]).unwrap();
                    }
                })
            }
            handle_event::spawn(None).unwrap();
        }
    }

    #[task(binds = UART0_IRQ, priority = 4, shared = [uart, action])]
    fn rx(mut c: rx::Context) {
        fn process_key(uart: &mut Uart) -> (u8, u8) {
            match uart.read() {
                Ok(key) => ((key >> 3) & 0b0011111, key & 0b00000111),
                Err(_) => (255, 255),
            }
        }

        // handle_event::spawn(Some(Event::Release(1, 1))).unwrap();

        handle_event::spawn(Some(Event::Press(3, 3))).unwrap();
        handle_event::spawn(Some(Event::Release(3, 3))).unwrap();

        c.shared.uart.lock(|uart| {
            if uart.uart_is_readable() {
                if let Some(action) = c.shared.action.inner {
                    let (x, y) = process_key(uart);

                    let key = match action {
                        ReceivedAction::Press => Event::Press(x, y),
                        ReceivedAction::Release => Event::Release(x, y),
                    };
                    handle_event::spawn(Some(key)).unwrap();

                    c.shared.action.update(None);
                } else {
                    let ident = match uart.read() {
                        Ok(ident) => ident,
                        Err(_) => ERROR_ID,
                    };

                    if ident == PRESS_ID {
                        c.shared.action.update(Some(ReceivedAction::Press));
                    } else if ident == RELEASE_ID {
                        c.shared.action.update(Some(ReceivedAction::Release));
                    } else {
                        c.shared.action.update(None);
                    };
                }
            }
        })
    }
}
