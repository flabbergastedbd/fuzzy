use crate::models::Task;

pub fn format_task<'a>(t: &'a Task) -> Vec<String> {
    vec![
        format!("{}", t.id),
        t.name.clone(),
        t.executor.clone().unwrap_or_default(),
        t.fuzz_driver.clone().unwrap_or_default(),
        format!("{}", t.active)
    ]
}
