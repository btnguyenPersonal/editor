use difference::Changeset;

pub struct DiffHistory {
    history: Vec<Changeset>,
    current: usize,
}

impl DiffHistory {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            current: 0,
        }
    }

    pub fn create_snapshot(&mut self, state: &mut Vec<String>) {
        let state_as_string = state.join("\n");
        let previous_state_as_string = self.get_current_state_as_string();
        let changeset = Changeset::new(&previous_state_as_string, &state_as_string, "\n");
        if self.history.len() != 0 && self.current != self.history.len() - 1 {
            self.history.truncate(self.current + 1);
        }
        self.history.push(changeset);
        self.current += 1;
    }

    fn get_current_state_as_string(&self) -> String {
        if self.history.len() == 0 {
            return String::new();
        }
        self.history[self.current - 1].diffs.iter().filter_map(|diff| {
            match diff {
                difference::Difference::Same(ref x) |
                difference::Difference::Add(ref x) => Some(x.clone()),
                _ => None,
            }
        }).collect::<Vec<String>>().join("\n")
    }
    
    fn reconstruct_state(&self, changeset: &Changeset) -> String {
        changeset.diffs.iter().map(|diff| {
            match diff {
                difference::Difference::Same(ref x) |
                difference::Difference::Add(ref x) => x.clone(),
                _ => String::new(),
            }
        }).collect::<Vec<String>>().join("\n")
    }

    pub fn undo(&mut self) -> Option<String> {
        if self.current > 0 {
            self.current -= 1;
        }
        self.get_state(self.current)
    }
    
    pub fn redo(&mut self) -> Option<String> {
        if self.current < self.history.len() - 1 {
            self.current += 1;
        }
        self.get_state(self.current)
    }

    fn get_state(&self, index: usize) -> Option<String> {
        self.history.get(index).map(|changeset| self.reconstruct_state(changeset))
    }
}
