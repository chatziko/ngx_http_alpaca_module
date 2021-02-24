/// Converst a Result<T,E> to Result<T,String> by calling .to_string() on the error
pub fn stringify_error<T, E:ToString>(res: Result<T,E>) -> Result<T, String> {
    match res {
        Ok(res) => return Ok(res),
        Err(e)  => return Err( e.to_string() ),
    }
}