use crate::models::{Task, Corpus};

pub fn format_task<'a>(t: &'a Task) -> Vec<String> {
    vec![
        format!("{}", t.id),
        t.name.clone(),
        t.executor.clone().unwrap_or_default(),
        t.fuzz_driver.clone().unwrap_or_default(),
        format!("{}", t.active)
    ]
}

pub fn format_corpus<'a>(c: &'a Corpus) -> Vec<String> {
    vec![
        format!("{}", c.id),
        c.checksum.clone(),
        c.label.clone()
    ]
}

pub fn print_corpora(corpora: Vec<Corpus>) {
    let corpora_heading = vec![
        "ID",
        "Checksum",
        "Label"
    ];
    let mut corpora_vec = Vec::new();
    for c in corpora.iter() {
        corpora_vec.push(format_corpus(c));
    }
    super::print_results(corpora_heading, corpora_vec);
}
