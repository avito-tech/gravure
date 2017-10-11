use config::Task;
use actions::*;
use errors::JobError;

use futures::sync::oneshot;
use futures::future::lazy;
use futures_pool::Sender;

pub struct Job {
    pub image_id: u64,
    pub image_path: String,
    pub task: Task,
    pub response: Option<oneshot::Sender<()>>,
}

impl Job {
    pub fn spawn(self, executor: Sender) {
        let Job {
            image_id,
            image_path,
            task,
            ..
        } = self;
        oneshot::spawn(lazy(move || {
            println!("QS JOB: {:?}", image_path);
            let mut imgd =
                try!(ImageData::new(image_path, image_id).map_err(|e| JobError::Image(e)));
            println!("QS IMAGE_DATA OK");

            for action in task.actions.iter() {
                imgd = try!(action.run(&mut imgd).map_err(|e| JobError::Action(e)));
            }
            Ok::<_, JobError>(())
        }),
                       &executor);
    }
}
