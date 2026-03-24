use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::{Duration, Instant};

lazy_static! {
    static ref PERF_DATA: Mutex<PerformanceData> = Mutex::new(PerformanceData::new());
}

#[derive(Debug, Default)]
struct PerformanceData {
    screenshot_times: Vec<Duration>,
    input_times: Vec<Duration>,
    ipc_times: Vec<Duration>,
}

impl PerformanceData {
    fn new() -> Self {
        Self::default()
    }

    fn add_screenshot_time(&mut self, duration: Duration) {
        self.screenshot_times.push(duration);
        if self.screenshot_times.len() > 100 {
            self.screenshot_times.remove(0);
        }
    }

    fn add_input_time(&mut self, duration: Duration) {
        self.input_times.push(duration);
        if self.input_times.len() > 100 {
            self.input_times.remove(0);
        }
    }

    fn add_ipc_time(&mut self, duration: Duration) {
        self.ipc_times.push(duration);
        if self.ipc_times.len() > 100 {
            self.ipc_times.remove(0);
        }
    }

    fn average_screenshot_time(&self) -> Duration {
        if self.screenshot_times.is_empty() {
            Duration::from_millis(0)
        } else {
            let total = self.screenshot_times.iter().sum::<Duration>();
            total / self.screenshot_times.len() as u32
        }
    }

    fn average_input_time(&self) -> Duration {
        if self.input_times.is_empty() {
            Duration::from_millis(0)
        } else {
            let total = self.input_times.iter().sum::<Duration>();
            total / self.input_times.len() as u32
        }
    }

    fn average_ipc_time(&self) -> Duration {
        if self.ipc_times.is_empty() {
            Duration::from_millis(0)
        } else {
            let total = self.ipc_times.iter().sum::<Duration>();
            total / self.ipc_times.len() as u32
        }
    }

    fn max_screenshot_time(&self) -> Duration {
        self.screenshot_times
            .iter()
            .cloned()
            .max()
            .unwrap_or(Duration::from_millis(0))
    }

    fn max_input_time(&self) -> Duration {
        self.input_times
            .iter()
            .cloned()
            .max()
            .unwrap_or(Duration::from_millis(0))
    }

    fn max_ipc_time(&self) -> Duration {
        self.ipc_times
            .iter()
            .cloned()
            .max()
            .unwrap_or(Duration::from_millis(0))
    }
}

pub struct PerfTimer {
    start: Instant,
    name: &'static str,
}

impl PerfTimer {
    pub fn new(name: &'static str) -> Self {
        Self {
            start: Instant::now(),
            name,
        }
    }

    pub fn stop(self) -> Duration {
        let duration = self.start.elapsed();

        if let Ok(mut data) = PERF_DATA.lock() {
            match self.name {
                "screenshot" => data.add_screenshot_time(duration),
                "input" => data.add_input_time(duration),
                "ipc" => data.add_ipc_time(duration),
                _ => {}
            }
        }

        duration
    }
}

pub fn get_perf_report() -> String {
    let data = PERF_DATA.lock().unwrap();

    format!(
        "Performance Report:\n\
        Screenshot:\n\
          - Average: {:.2?}\n\
          - Max: {:.2?}\n\
          - Samples: {}\n\
        Input:\n\
          - Average: {:.2?}\n\
          - Max: {:.2?}\n\
          - Samples: {}\n\
        IPC:\n\
          - Average: {:.2?}\n\
          - Max: {:.2?}\n\
          - Samples: {}\n",
        data.average_screenshot_time(),
        data.max_screenshot_time(),
        data.screenshot_times.len(),
        data.average_input_time(),
        data.max_input_time(),
        data.input_times.len(),
        data.average_ipc_time(),
        data.max_ipc_time(),
        data.ipc_times.len()
    )
}

#[allow(unused_macros)]
macro_rules! measure_time {
    ($name:expr, $code:block) => {{
        let timer = PerfTimer::new($name);
        let result = $code;
        let duration = timer.stop();
        (result, duration)
    }};
}

#[allow(unused_imports)]
pub(crate) use measure_time;
