use seatrial_lib::binary::{self, Spec, Stop};

pub async fn execute(spec: &Spec, stop: Stop) -> Result<(), String> {
    let output = binary::run(spec, stop).await?;
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Err(format!("scene command {}", output.status));
    }
    Ok(())
}
