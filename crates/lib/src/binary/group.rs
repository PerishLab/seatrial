pub struct Group {
    pid: Option<u32>,
    tree: bool,
}

impl Group {
    pub fn new(pid: u32, tree: bool) -> Self {
        Self {
            pid: Some(pid),
            tree,
        }
    }

    pub fn release(&mut self) {
        if !self.tree {
            self.pid = None;
        }
    }

    pub fn kill(&mut self) -> Result<(), String> {
        let Some(pid) = self.pid else {
            return Ok(());
        };
        let pid = i32::try_from(pid).map_err(|error| error.to_string())?;
        let target = match self.tree {
            true => -pid,
            false => pid,
        };
        let status = unsafe { libc::kill(target, libc::SIGKILL) };
        if status == 0 {
            self.release();
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            self.release();
            return Ok(());
        }
        Err(format!("terminate process scope {target}: {error}"))
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}
