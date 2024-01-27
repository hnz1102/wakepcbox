use log::*;
use std::{thread, time::Duration, sync::Arc, sync::Mutex, sync::atomic::AtomicBool};
use esp_idf_hal::{gpio::*};
use std::sync::atomic::Ordering;
use std::time::SystemTime;

type PINDRIVER4 = Box<PinDriver<'static, esp_idf_hal::gpio::Gpio4, esp_idf_hal::gpio::Input>>; // GPIO4
type PINDRIVER5 = Box<PinDriver<'static, esp_idf_hal::gpio::Gpio5, esp_idf_hal::gpio::Input>>; // GPIO5
type PINDRIVER6 = Box<PinDriver<'static, esp_idf_hal::gpio::Gpio6, esp_idf_hal::gpio::Input>>; // GPIO6
type PINDRIVER9 = Box<PinDriver<'static, esp_idf_hal::gpio::Gpio9, esp_idf_hal::gpio::Input>>; // GPIO9
type PINDRIVER10 = Box<PinDriver<'static, esp_idf_hal::gpio::Gpio10, esp_idf_hal::gpio::Input>>; // GPIO10

static GPIO4_FLAG: AtomicBool = AtomicBool::new(false);
static GPIO5_FLAG: AtomicBool = AtomicBool::new(false);
static GPIO6_FLAG: AtomicBool = AtomicBool::new(false);
static GPIO9_FLAG: AtomicBool = AtomicBool::new(false);
static GPIO10_FLAG: AtomicBool = AtomicBool::new(false);

const GARD_TIME: u128 = 30;    // if the time difference is less than 30ms, ignore the interrupt
const KEY_SLEEP_TIME: u64 = 10; // scan the key every 10ms

#[allow(dead_code)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, Copy)]
pub enum KeyEvent {
    UpKeyDown,
    UpKeyUp,
    DownKeyDown,
    DownKeyUp,
    LeftKeyDown,
    LeftKeyUp,
    RightKeyDown,
    RightKeyUp,
    CenterKeyDown,
    CenterKeyUp,
}

struct KeyState {
    up: bool,
    up_count: u32,
    down: bool,
    down_count: u32,
    left: bool,
    left_count: u32,
    right: bool,
    right_count: u32,
    center: bool,
    center_count: u32,
    key_envet: Vec<KeyEvent>,
    key_sleep: bool,
}

pub struct KeySwitch {
    state: Arc<Mutex<KeyState>>
}

#[allow(dead_code)]
impl KeySwitch {
    pub fn new() -> KeySwitch {
        KeySwitch { state: Arc::new(Mutex::new(
            KeyState { 
                up :    false,  up_count:       0,
                down:   false,  down_count:     0,
                left:   false,  left_count:     0,
                right:  false,  right_count:    0,
                center: false,  center_count:   0,
                key_envet: Vec::new(),
                key_sleep: false,
            }))}
    }

    // UP: GPIO4
    pub fn gpio4_interrupt_handler() {
        GPIO4_FLAG.store(true, Ordering::Relaxed);
    }

    // DOWN: GPIO5
    pub fn gpio5_interrupt_handler() {
        GPIO5_FLAG.store(true, Ordering::Relaxed);
    }

    // RIGHT: GPIO6
    pub fn gpio6_interrupt_handler() {
        GPIO6_FLAG.store(true, Ordering::Relaxed);
    }

    // CENTER: GPIO9
    pub fn gpio9_interrupt_handler() {
        GPIO9_FLAG.store(true, Ordering::Relaxed);
    }

    // LEFT: GPIO10
    pub fn gpio10_interrupt_handler() {
        GPIO10_FLAG.store(true, Ordering::Relaxed);
    }

    pub fn start(&mut self,
            mut gpio4_sig : PINDRIVER4,
            mut gpio5_sig : PINDRIVER5,
            mut gpio6_sig : PINDRIVER6,
            mut gpio9_sig : PINDRIVER9,
            mut gpio10_sig : PINDRIVER10)
    {
        let state = self.state.clone();
        let _th = thread::spawn(move || {
            info!("Start Switch Read Thread.");            
            gpio4_sig.set_pull(Pull::Up).unwrap();
            gpio5_sig.set_pull(Pull::Up).unwrap();
            gpio6_sig.set_pull(Pull::Up).unwrap();
            gpio9_sig.set_pull(Pull::Up).unwrap();
            gpio10_sig.set_pull(Pull::Up).unwrap();

            gpio4_sig.set_interrupt_type(InterruptType::AnyEdge).unwrap();
            gpio5_sig.set_interrupt_type(InterruptType::AnyEdge).unwrap();
            gpio6_sig.set_interrupt_type(InterruptType::AnyEdge).unwrap();
            gpio9_sig.set_interrupt_type(InterruptType::AnyEdge).unwrap();
            gpio10_sig.set_interrupt_type(InterruptType::AnyEdge).unwrap();

            unsafe {
                gpio4_sig.subscribe(KeySwitch::gpio4_interrupt_handler).unwrap();
                gpio5_sig.subscribe(KeySwitch::gpio5_interrupt_handler).unwrap();
                gpio6_sig.subscribe(KeySwitch::gpio6_interrupt_handler).unwrap();
                gpio9_sig.subscribe(KeySwitch::gpio9_interrupt_handler).unwrap();
                gpio10_sig.subscribe(KeySwitch::gpio10_interrupt_handler).unwrap();
            }        

            gpio4_sig.enable_interrupt().unwrap();
            gpio5_sig.enable_interrupt().unwrap();
            gpio6_sig.enable_interrupt().unwrap();
            gpio9_sig.enable_interrupt().unwrap();
            gpio10_sig.enable_interrupt().unwrap();

            // caluculate the time difference between the last interrupt and now
            // if the time differrence is less than 1sec, igonore the interrupt
            let mut last_interrupt_time_up = SystemTime::now();
            let mut last_interrupt_time_down = SystemTime::now();
            let mut last_interrupt_time_right = SystemTime::now();
            let mut last_interrupt_time_center = SystemTime::now();
            let mut last_interrupt_time_left = SystemTime::now();
            loop {
                let mut lck = state.lock().unwrap();
                if lck.key_sleep == true {
                    lck.key_envet.clear();
                    drop(lck);
                    thread::sleep(Duration::from_millis(KEY_SLEEP_TIME));
                    continue;
                }

                // UP: GPIO4
                if GPIO4_FLAG.load(Ordering::Relaxed) {
                    match last_interrupt_time_up.elapsed(){
                        Ok(elapsed) => {
                            if elapsed.as_millis() > GARD_TIME {
                                if lck.up == false {
                                    lck.up = true;
                                    lck.up_count = 0;
                                    lck.key_envet.push(KeyEvent::UpKeyDown);
                                    // info!("PUSH_NOTIFICATION_UP_KEY_DOWN");
                                }
                                else {
                                    lck.up = false;
                                    lck.up_count = match last_interrupt_time_up.elapsed() {
                                        Ok(elapsed) => elapsed.as_millis() as u32,
                                        Err(e) => {
                                            error!("Error: {:?}", e);
                                            0
                                        }
                                    };
                                    lck.key_envet.push(KeyEvent::UpKeyUp);
                                    // info!("PUSH_NOTIFICATION_UP_KEY_UP {}", lck.up_count);
                                }
                                last_interrupt_time_up = SystemTime::now();
                            }
                        },
                        Err(e) => {
                            error!("Error: {:?}", e);
                        }
                    }
                    GPIO4_FLAG.store(false, Ordering::Relaxed);
                    gpio4_sig.enable_interrupt().unwrap();
                }
                // DOWN: GPIO5
                if GPIO5_FLAG.load(Ordering::Relaxed) {
                    match last_interrupt_time_down.elapsed() {
                        Ok(elapsed) => {
                            if elapsed.as_millis() > GARD_TIME {
                                if lck.down == false {
                                    lck.down = true;
                                    lck.down_count = 0;
                                    lck.key_envet.push(KeyEvent::DownKeyDown);
                                    // info!("PUSH_NOTIFICATION_DOWN_KEY_DOWN");
                                }
                                else {
                                    lck.down = false;
                                    lck.down_count = match last_interrupt_time_down.elapsed() {
                                        Ok(elapsed) => elapsed.as_millis() as u32,
                                        Err(e) => {
                                            error!("Error: {:?}", e);
                                            0
                                        }
                                    };
                                    lck.key_envet.push(KeyEvent::DownKeyUp);
                                    // info!("PUSH_NOTIFICATION_DOWN_KEY_UP {}", lck.down_count);
                                }
                                last_interrupt_time_down = SystemTime::now();
                            }
                        },
                        Err(e) => {
                            error!("Error: {:?}", e);
                        }
                    }
                    GPIO5_FLAG.store(false, Ordering::Relaxed);
                    gpio5_sig.enable_interrupt().unwrap();
                }
                // RIGHT: GPIO6
                if GPIO6_FLAG.load(Ordering::Relaxed) {
                    match last_interrupt_time_right.elapsed() {
                        Ok(elapsed) => {
                            if elapsed.as_millis() > GARD_TIME {
                                if lck.right == false {
                                    lck.right = true;
                                    lck.right_count = 0;
                                    lck.key_envet.push(KeyEvent::RightKeyDown);
                                    // info!("PUSH_NOTIFICATION_RIGHT_KEY_DOWN");
                                }
                                else {
                                    lck.right = false;
                                    lck.right_count = match last_interrupt_time_right.elapsed() {
                                        Ok(elapsed) => elapsed.as_millis() as u32,
                                        Err(e) => {
                                            error!("Error: {:?}", e);
                                            0
                                        }
                                    };
                                    lck.key_envet.push(KeyEvent::RightKeyUp);
                                    // info!("PUSH_NOTIFICATION_RIGHT_KEY_UP {}", lck.right_count);
                                }
                                last_interrupt_time_right = SystemTime::now();
                            }
                        },
                        Err(e) => {
                            error!("Error: {:?}", e);
                        }
                    }
                    GPIO6_FLAG.store(false, Ordering::Relaxed);
                    gpio6_sig.enable_interrupt().unwrap();
                }
                // CENTER: GPIO9
                if GPIO9_FLAG.load(Ordering::Relaxed) {
                    match last_interrupt_time_center.elapsed() {
                        Ok(elapsed) => {
                            if elapsed.as_millis() > GARD_TIME {
                                if lck.center == false {
                                    lck.center = true;
                                    lck.center_count = 0;
                                    lck.key_envet.push(KeyEvent::CenterKeyDown);
                                    // info!("PUSH_NOTIFICATION_CENTER_KEY_DOWN");
                                }
                                else {
                                    lck.center = false;
                                    lck.center_count = match last_interrupt_time_center.elapsed() {
                                        Ok(elapsed) => elapsed.as_millis() as u32,
                                        Err(e) => {
                                            error!("Error: {:?}", e);
                                            0
                                        }
                                    };
                                    lck.key_envet.push(KeyEvent::CenterKeyUp);
                                    // info!("PUSH_NOTIFICATION_CENTER_KEY_UP {}", lck.center_count);
                                }
                                last_interrupt_time_center = SystemTime::now();
                            }
                        },
                        Err(e) => {
                            error!("Error: {:?}", e);
                        }
                    }
                    GPIO9_FLAG.store(false, Ordering::Relaxed);
                    gpio9_sig.enable_interrupt().unwrap();
                }
                // LEFT: GPIO10
                if GPIO10_FLAG.load(Ordering::Relaxed) {
                    match last_interrupt_time_left.elapsed() {
                        Ok(elapsed) => {
                            if elapsed.as_millis() > GARD_TIME {
                                if lck.left == false {
                                    lck.left = true;
                                    lck.left_count = 0;
                                    lck.key_envet.push(KeyEvent::LeftKeyDown);
                                    // info!("PUSH_NOTIFICATION_LEFT_KEY_DOWN");
                                }
                                else {
                                    lck.left = false;
                                    lck.left_count = match last_interrupt_time_left.elapsed() {
                                        Ok(elapsed) => elapsed.as_millis() as u32,
                                        Err(e) => {
                                            error!("Error: {:?}", e);
                                            0
                                        }
                                    };
                                    lck.key_envet.push(KeyEvent::LeftKeyUp);
                                    // info!("PUSH_NOTIFICATION_LEFT_KEY_UP {}", lck.left_count);
                                }
                                last_interrupt_time_left = SystemTime::now();
                            }
                        },
                        Err(e) => {
                            error!("Error: {:?}", e);
                        }
                    }
                    GPIO10_FLAG.store(false, Ordering::Relaxed);
                    gpio10_sig.enable_interrupt().unwrap();
                }
                drop(lck);
                thread::sleep(Duration::from_millis(KEY_SLEEP_TIME));
            }
        });
    }

    pub fn get_current_button_state(&mut self, button: Key) -> bool
    {
        let lock= self.state.lock().unwrap();
        match button {
            Key::Up => {
                let ret = lock.up;
                ret
            },
            Key::Down => {
                let ret = lock.down;
                ret
            },
            Key::Left => {
                let ret = lock.left;
                ret
            },
            Key::Right => {
                let ret = lock.right;
                ret
            },
            Key::Center => {
                let ret = lock.center;
                ret
            },
        }
    }

    pub fn get_button_press_time(&mut self, button: Key) -> u32
    {
        let lock= self.state.lock().unwrap();
        match button {
            Key::Up => {
                let ret = lock.up_count;
                ret
            },
            Key::Down => {
                let ret = lock.down_count;
                ret
            },
            Key::Left => {
                let ret = lock.left_count;
                ret
            },
            Key::Right => {
                let ret = lock.right_count;
                ret
            },
            Key::Center => {
                let ret = lock.center_count;
                ret
            },
        }
    }

    pub fn clear_all_button_event(&mut self)
    {
        let mut lock= self.state.lock().unwrap();
        lock.key_envet.clear();
    }

    pub fn get_key_event_and_clear(&mut self) -> Vec<KeyEvent>
    {
        let mut lock= self.state.lock().unwrap();
        let ret = lock.key_envet.clone();
        lock.key_envet.clear();
        ret
    }

    pub fn set_key_sleep(&mut self, sleep: bool)
    {
        let mut lock= self.state.lock().unwrap();
        lock.key_sleep = sleep;
    }
}
