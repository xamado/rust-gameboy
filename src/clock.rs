// use std::future::Future;
// use std::pin::Pin;
// use std::task::{Poll, Context};

// mod waker;

// pub enum State {
//     Halted,
//     Running
// }

// pub struct ClockTask {
//     state: State
// }

// impl ClockTask {
//     pub fn waiter<'a>(&'a mut self) -> Waiter<'a> {
//         Waiter { task: self }
//     }
// }

// pub struct Waiter<'a> {
//     task: &'a mut ClockTask
// }

// impl<'a> Future for Waiter<'a> {
//     type Output = ();

//     fn poll(mut self: Pin<&mut Self>, _ctx: &mut Context) -> Poll<Self::Output> {
//         match self.task.state {
//             State::Halted => {
//                 self.task.state = State::Running;
//                 Poll::Ready(())
//             }
//             State::Running => {
//                 self.task.state = State::Halted;
//                 Poll::Pending
//             }
//         }
//     }
// }

// pub struct Clock {
//     tasks: Vec<Pin<Box<dyn Future<Output=()>>>>
// }

// impl Clock {
//     pub fn new() -> Self {
//         Clock {
//             tasks: Vec::new()
//         }
//     }

//     pub fn push<C, F>(&mut self, closure: C) where F: Future<Output=()> + 'static, C: FnOnce(ClockTask) -> F { 
//         let task = ClockTask { state: State::Running };
//         self.tasks.push(Box::pin(closure(task)));
//     }

//     pub fn tick(&mut self) {
//         let waker = waker::create();
//         let mut context = Context::from_waker(&waker);
        
//         for task in self.tasks.iter_mut() {
//             match task.as_mut().poll(&mut context) {
//                 _ => {}
//             }
//         }
//     }
// }