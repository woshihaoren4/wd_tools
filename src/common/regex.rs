pub trait SimpleRegexMatch {
    fn regex(&self, re: &str) -> anyhow::Result<Vec<&str>>;
}

impl SimpleRegexMatch for str {
    fn regex(&self, re: &str) -> anyhow::Result<Vec<&str>> {
        let re = regex::Regex::new(re)?;
        let mut list = vec![];
        for (_, [id]) in re.captures_iter(self).map(|c| c.extract()) {
            list.push(id)
        }
        Ok(list)
    }
}
impl SimpleRegexMatch for String {
    fn regex(&self, re: &str) -> anyhow::Result<Vec<&str>> {
        self.as_str().regex(re)
    }
}
