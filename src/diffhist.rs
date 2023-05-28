pub struct DiffHistory {
    undo_stack: Vec<(Vec<String>, (usize, usize))>,
    redo_stack: Vec<(Vec<String>, (usize, usize))>,
    current_doc: Vec<String>,
    current_pos: (usize, usize),
}


impl DiffHistory {
    pub fn new(doc: Vec<String>) -> Self {
        DiffHistory {
            current_doc: doc,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_pos: (0, 0),
        }
    }

    pub fn make_change(&mut self, new_doc: Vec<String>, new_pos: (usize, usize)) {
        self.undo_stack.push((self.current_doc.clone(), self.current_pos));
        self.redo_stack.clear();
        self.current_doc = new_doc;
        self.current_pos = new_pos;
    }

    pub fn undo(&mut self) -> Option<(Vec<String>, (usize, usize))> {
        if let Some((old_doc, old_pos)) = self.undo_stack.pop() {
            self.redo_stack.push((self.current_doc.clone(), self.current_pos));
            self.current_doc = old_doc;
            let pos = self.current_pos;
            self.current_pos = old_pos;
            return Some((self.current_doc.clone(), pos));
        }
        None
    }

    pub fn redo(&mut self) -> Option<(Vec<String>, (usize, usize))> {
        if let Some((old_doc, old_pos)) = self.redo_stack.pop() {
            self.undo_stack.push((self.current_doc.clone(), self.current_pos));
            self.current_doc = old_doc;
            self.current_pos = old_pos;
            return Some((self.current_doc.clone(), self.current_pos));
        }
        None
    }
}