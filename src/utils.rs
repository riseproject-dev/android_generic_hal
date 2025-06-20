pub fn read_sysfs_file(path: &str) -> std::io::Result<String> {
    let mut data = std::fs::read_to_string(&path)?;

    data.pop(); // Remove trailing '\n'

    Ok(data)
}
