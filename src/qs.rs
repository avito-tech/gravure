use config::Task;
use actions::*;
use errors::JobError;

use std::sync::Arc;

use futures::sync::oneshot;
use futures::Future;
use futures::future::lazy;
use futures_pool::Sender;
use slog_scope;

pub struct Job {
    pub image_id: u64,
    pub image_path: String,
    pub task: Task,
    pub response: Option<oneshot::Sender<()>>,
    pub client: Arc<String>,
}

impl Job {
    pub fn spawn(self, executor: Sender) {
        let run = move || {
            slog_scope::scope(&slog_scope::logger()
                                   .new(slog_o!("scope" => "job action", "id"=>self.image_id, "path"=>self.image_path.clone(), "client"=>self.client.clone())),
                              || self.run().map_err(|e| warn!("job error {}", e)))
        };
        oneshot::spawn(lazy(run).map(|response| if let Some(response) = response {
                                         response
                                             .send(())
                                             .unwrap_or_else(|_| {
                                                                 info!("job response not set");
                                                             });
                                     }),
                       &executor);
    }

    fn run(self) -> Result<Option<oneshot::Sender<()>>, JobError> {
        let Job {
            image_id,
            image_path,
            task,
            response,
            ..
        } = self;
        let mut imgd = ImageData::new(image_path, image_id)
            .map_err(|e| JobError::Image(e))?;

        for action in task.actions.iter() {
            imgd = try!(action.run(&mut imgd).map_err(|e| JobError::Action(e)));
        }
        Ok(response)
    }
}
