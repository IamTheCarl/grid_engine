
//! The core scheduler for our engine.
//! These are all features related to managing the engine's time. You schedule
//! events and then wait for the time they should be processed.

use log;
use std::time::{Duration, SystemTime};

use std::{
    collections::BinaryHeap,
    error,
    fmt,
    sync::mpsc,
    ops,
    cmp,
};

use threadpool::ThreadPool;

/// Simulation time. Is tracked in milliseconds.
/// Although you can operate on it using std::time::Duration, this struct only
/// has precision in milliseconds. That means that if you set the microseconds
/// or nanoseconds of the duration, they will be truncated from the final product.
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq)]
pub struct Time {
    time_ms: u64,
}

impl Time {
    /// Construct a new time struct from the provided time in milliseconds.
    pub fn from_ms(ms: u64) -> Time {
        Time { time_ms: ms }
    }

    // TODO display formatters.
    // TODO get delta by subtracting another
}

impl ops::Add<Duration> for Time {
    type Output = Time;

    fn add(mut self, other: Duration) -> Self {
        self.time_ms += other.as_millis() as u64;
        self
    }
}

impl ops::Sub<Duration> for Time {
    type Output = Time;

    fn sub(mut self, other: Duration) -> Self {
        self.time_ms -= other.as_millis() as u64;
        self
    }
}

impl ops::AddAssign<Duration> for Time {
    fn add_assign(&mut self, delta: Duration) {
        self.time_ms += delta.as_millis() as u64;
    }
}

impl ops::SubAssign<Duration> for Time {
    fn sub_assign(&mut self, delta: Duration) {
        self.time_ms -= delta.as_millis() as u64;
    }
}

/// A closure to be called when an event's time comes.
/// It will be passed the scheduler that has called it so future or dependent events can be scheduled.
pub type EventCallback = dyn FnOnce(&SchedulerProxy) + 'static + Send;

/// A scheduled event to be ran by the Scheduler.
pub struct Event {
    time: Time,
    callback: Box<EventCallback>,
}

impl Event {
    /// Create a new event that can be added to a scheduler for execution.
    pub fn new<F: FnOnce(&SchedulerProxy) + 'static + Send>(time: Time, callback: F) -> Event {
        // We wrap it in a box here so we can later change our internal representation as we see fit.
        // What I'm saying here is that I'd like to not be using a box here.
        Event { time, callback: Box::new(callback) }
    }

    /// Get the time the event should happen at.
    pub fn time(&self) -> Time {
        self.time
    }

    fn run_callback(self, proxy: &SchedulerProxy) {
        let callback = self.callback;
        callback(proxy);
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

impl Eq for Event {}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

/// An enum for the different kinds of recoverable errors the scheduler can experience.
#[derive(Debug)]
pub enum SchedulerError {
    /// Attempted to schedule an event to happen int he past.
    ScheduledInPast,
}

impl error::Error for SchedulerError {}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchedulerError::ScheduledInPast => write!(f, "Attempted to schedule an event to happen in the past."),
        }
    }
}

/// Runs tasks roughly in order, but also in parallel.
/// The order they are ran is determined by the time they are scheduled to run at.
/// This scheduler does its best to run the tasks at their scheduled times.
pub struct Scheduler {
    priority_queue: BinaryHeap<cmp::Reverse<Event>>,
    event_tx: mpsc::Sender<Event>,
    event_rx: mpsc::Receiver<Event>,
    current_time: Time,
    thread_pool: ThreadPool,
}

impl Scheduler {
    /// Create a new scheduler.
    /// The scheduler uses an internal thread pool. It is recommended that the number of threads
    /// used equal the number of threads the hardware natively supports.
    pub fn new(num_threads: usize) -> Scheduler {
        let (event_tx, event_rx) = mpsc::channel();
        Scheduler { priority_queue: BinaryHeap::new(), event_tx, event_rx, current_time: Time::from_ms(0), thread_pool: ThreadPool::new(num_threads) }
    }

    /// Get the current time of the simulation.
    pub fn now(&self) -> Time {
        self.current_time
    }

    /// Will cause the scheduler to run events over a certain duration of time. More than
    /// likely, all of the events will be processed in less time than the duration covers.
    /// When this happens, a duration is returned for how long the scheduler recommends
    /// you sleep until running a tick cycle again.
    pub fn tick(&mut self, delta: Duration) -> Duration {
        let now = SystemTime::now();

        // Update the current time and then get it into a local register.
        self.current_time += delta;
        let current_time = self.current_time;

        let panic_count = self.thread_pool.panic_count();

        loop {
            loop {
                let next = self.priority_queue.pop();
                if let Some(next) = next {
                    let next = next.0;
                    // Okay so we have something.
                    if next.time <= current_time {
                        // We execute this one.

                        // TODO creating a new one of these may not be such a good idea for performance.
                        // We may need to implement our own threadpool to really do this efficiently.
                        let proxy = SchedulerProxy {
                            event_tx: self.event_tx.clone(),
                            current_time: next.time()
                        };

                        self.thread_pool.execute(move || {
                            next.run_callback(&proxy);
                        });
                    } else {
                        // Too early for this one? Then we've emptied the queue of what we can execute.

                        // Put that thing back where it came from or so help me.
                        self.priority_queue.push(cmp::Reverse(next));

                        // Break out of this loop and see if we can load up more.
                        break;
                    }
                } else {
                    // Queue is empty. We're done.
                    break;
                }
            }


            fn add_events(us: &mut Scheduler) -> bool {
                let event_count = us.priority_queue.len();
            
                // Fill up the queue with new events.
                for event in us.event_rx.try_iter() {
                    println!("Add Event");
                    us.priority_queue.push(cmp::Reverse(event));
                }

                event_count != us.priority_queue.len()
            }

            // Try and add more events if you can.
            if !add_events(self) {
                // Nothing was added, but the threads may try to add more while they're processing.
                // Wait for them to finish and then try again.
                self.thread_pool.join();

                if !add_events(self) {
                    // Nothing was added. Time to break out.
                    println!("Break.");
                    break;
                }
            }
        }


        // Make sure everything is done.
        self.thread_pool.join();

        // Because we made sure all the jobs finished first, we know this is ready.
        let new_panic_count = self.thread_pool.panic_count();
        if panic_count > new_panic_count {
            log::error!("{} threads panicked this tick.", new_panic_count);
        }

        let elapsed = now.elapsed();
        match elapsed {
            Ok(elapsed) => elapsed,
            Err(error) => {
                log::warn!("Failed to get elapsed time while processing tick. Cause: {}", error);
                // If we don't know how long we waited, then just wait the full time.
                delta
            }
        }
    }

    /// Schedule an event to happen. Will fail if the event is set to happen in the past.
    /// This function will not wake the processing thread from a sleep state, so there's a
    /// chance your event could be processed late if it was scheduled outside of the event
    /// processing threads. It will however, always be processed before any other events
    /// that were meant to happen after it.
    pub fn schedule_event(&self, event: Event) -> Result<(), SchedulerError> {
        if event.time() >= self.now() {
            self.event_tx.send(event)
                .expect("Scheduler receiver was disposed too early.");
            Ok(())
        } else {
            Err(SchedulerError::ScheduledInPast)
        }
    }
}

/// We cannot give a direct access to the scheduler to every event handler, so we
/// give a proxy for the case that they do wish to schedule more events.
pub struct SchedulerProxy {
    event_tx: mpsc::Sender<Event>,
    current_time: Time,
}

impl SchedulerProxy {
    /// Schedule an event to happen. Will fail if the event is set to happen in the past.
    /// This function will not wake the processing thread from a sleep state, so there's a
    /// chance your event could be processed late if it was scheduled outside of the event
    /// processing threads. It will however, always be processed before any other events
    /// that were meant to happen after it.
    pub fn schedule_event(&self, event: Event) -> Result<(), SchedulerError> {
        if event.time() >= self.current_time {
            self.event_tx.send(event)
                .expect("Scheduler receiver was disposed too early.");
            Ok(())
        } else {
            Err(SchedulerError::ScheduledInPast)
        }
    }

    /// Returns the time that this proxy is valid for.
    pub fn now(&self) -> Time {
        self.current_time
    }
}

#[cfg(test)]
mod test_scheduler {
    use super::*;

    #[test]
    fn cycle_one_thread_no_events() {
        let mut scheduler = Scheduler::new(1);
        scheduler.tick(Duration::from_secs(1));
    }

    #[test]
    fn cycle_two_threads_no_events() {
        let mut scheduler = Scheduler::new(2);
        scheduler.tick(Duration::from_secs(1));
    }

    #[test]
    fn cycle_one_thread_one_event() {
        let mut scheduler = Scheduler::new(1);
        scheduler.schedule_event(Event::new(scheduler.now() + Duration::from_secs(1), |_p| {})).unwrap();

        scheduler.tick(Duration::from_secs(1));
    }

    #[test]
    fn cycle_one_thread_two_events() {
        let mut scheduler = Scheduler::new(1);
        scheduler.schedule_event(Event::new(scheduler.now() + Duration::from_secs(1), |_p| {})).unwrap();
        scheduler.schedule_event(Event::new(scheduler.now() + Duration::from_secs(1), |_p| {})).unwrap();

        scheduler.tick(Duration::from_secs(1));
    }

    #[test]
    fn cycle_two_threads_one_event() {
        let mut scheduler = Scheduler::new(2);
        scheduler.schedule_event(Event::new(scheduler.now() + Duration::from_secs(1), |_p| {})).unwrap();

        scheduler.tick(Duration::from_secs(1));
    }

    #[test]
    fn cycle_two_threads_two_events() {
        let mut scheduler = Scheduler::new(2);
        scheduler.schedule_event(Event::new(scheduler.now() + Duration::from_secs(1), |_p| {})).unwrap();
        scheduler.schedule_event(Event::new(scheduler.now() + Duration::from_secs(1), |_p| {})).unwrap();

        scheduler.tick(Duration::from_secs(1));
    }

    #[test]
    fn schedule_event_in_the_past() {
        let mut scheduler = Scheduler::new(1);
        let the_past = scheduler.now();

        // Tick us into the future.
        scheduler.tick(Duration::from_secs(1));

        let result = scheduler.schedule_event(Event::new(the_past, |_p| {}));
        if let Err(error) = result {
            #[allow(unreachable_patterns)]
            match error {
                SchedulerError::ScheduledInPast => {
                    // This is correct. No panic. You passed the test.
                },
                _ => {
                    panic!("Wrong error type.")
                }
            }
        } else {
            panic!("Didn't fail when we should have.");
        }
    }

    #[test]
    fn tick_into_the_future() {
        let mut scheduler = Scheduler::new(1);
        let the_past = scheduler.now();

        // Tick us into the future.
        scheduler.tick(Duration::from_secs(1));

        assert!(the_past < scheduler.now());
    }

    #[test]
    fn check_order() {
        // It is important we only use one thread here, so that we get a consistent output.
        let mut scheduler = Scheduler::new(1);

        let (tx, rx) = mpsc::channel();

        let tx_copy = tx.clone();
        scheduler.schedule_event(Event::new(scheduler.now() + Duration::from_secs(1), move |_p| {
            println!("A");
            tx_copy.send(1).unwrap();
        })).unwrap();

        let tx_copy = tx.clone();
        scheduler.schedule_event(Event::new(scheduler.now() + Duration::from_secs(2), move |_p| {
            println!("B");
            tx_copy.send(2).unwrap();
        })).unwrap();

        let tx_copy = tx.clone();
        scheduler.schedule_event(Event::new(scheduler.now() + Duration::from_secs(3), move |_p| {
            println!("C");
            tx_copy.send(3).unwrap();
        })).unwrap();

        let tx_copy = tx.clone();
        scheduler.schedule_event(Event::new(scheduler.now() + Duration::from_secs(4), move |_p| {
            println!("D");
            tx_copy.send(4).unwrap();
        })).unwrap();

        scheduler.tick(Duration::from_secs(5));

        let mut numbers = Vec::new();

        for number in rx.try_iter() {
            numbers.push(number);
        }

        // Now check that they ran in the right order.
        assert_eq!(&numbers[..], [1, 2, 3, 4]);
    }

    #[test]
    fn cascading_events() {
        // It is important we only use one thread here, so that we get a consistent output.
        let mut scheduler = Scheduler::new(1);

        let (tx, rx) = mpsc::channel();

        let tx_copy_1 = tx.clone();
        let tx_copy_2 = tx.clone();
        let tx_copy_3 = tx.clone();
        let tx_copy_4 = tx.clone();
        scheduler.schedule_event(Event::new(scheduler.now() + Duration::from_secs(1), move |p| {
            println!("A");
            tx_copy_1.send(1).unwrap();
            p.schedule_event(Event::new(p.now() + Duration::from_secs(1), move |p| {
                println!("B");
                tx_copy_2.send(2).unwrap();
                p.schedule_event(Event::new(p.now() + Duration::from_secs(1), move |p| {
                    println!("C");
                    tx_copy_3.send(3).unwrap();
                    p.schedule_event(Event::new(p.now() + Duration::from_secs(1), move |_p| {
                        println!("D");
                        tx_copy_4.send(4).unwrap();
                    })).unwrap();
                })).unwrap();
            })).unwrap();
        })).unwrap();

        scheduler.tick(Duration::from_secs(5));

        let mut numbers = Vec::new();

        for number in rx.try_iter() {
            numbers.push(number);
        }

        // Now check that they ran in the right order.
        assert_eq!(&numbers[..], [1, 2, 3, 4]);
    }
}