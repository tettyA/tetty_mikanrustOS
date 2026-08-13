pub struct Queue<T> {
    data: [Option<T>; 256],
    pop_index: usize,
    push_index: usize,
}

impl<T> Queue<T>
where
    T: Copy + Clone,
{
    pub const fn new() -> Self {
        Self {
            data: [None; 256],
            pop_index: 0,
            push_index: 0,
        }
    }
    pub fn push(&mut self, data: T) -> Result<(), ()> {
        let next_push = (self.push_index + 1) % self.data.len();
        if next_push == self.pop_index {
            return Err(());
        }

        self.data[self.push_index] = Some(data);
        self.push_index = next_push;
        return Ok(());
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.pop_index == self.push_index {
            return None;
        }
        let ret = self.data[self.pop_index];
        self.data[self.pop_index] = None;
        self.pop_index = (self.pop_index + 1) % self.data.len();
        return ret;
    }
    pub fn count(&self) -> usize {
        if self.pop_index > self.push_index {
            self.push_index + self.data.len() - self.pop_index
        } else {
            self.push_index - self.pop_index
        }
    }
}
