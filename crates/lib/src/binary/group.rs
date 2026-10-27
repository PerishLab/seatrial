pub struct Group(pub u32);

impl Group {
    pub fn kill(&self) -> Result<(), String> {
        let pid = i32::try_from(self.0).map_err(|error| error.to_string())?;
        let status = unsafe { libc::kill(-pid, libc::SIGKILL) };
        if status == 0 {
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(());
        }
        Err(format!("terminate process group {pid}: {error}"))
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}
