use serde::{Serialize, Serializer};
use std::fmt::Display;

/// Vk API List helper
/// Serialize any iterable struct with `ToString` items to string separated by comma.
/// Example:
/// ```rust
/// use vkclient::List;
/// assert_eq!(List(vec![1, 2, 3]).to_string(), "1,2,3".to_string());
/// ```
#[derive(Copy, Clone, Debug, Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct List<I>(pub I);

impl<Item, Iter> Serialize for List<Iter>
where
    Item: Display,
    for<'a> &'a Iter: IntoIterator<Item = &'a Item>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<Item, Iter> Display for List<Iter>
where
    Item: Display,
    for<'a> &'a Iter: IntoIterator<Item = &'a Item>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.0.into_iter();

        match iter.next() {
            None => return Ok(()),
            Some(i) => write!(f, "{}", i)?,
        };

        for i in iter {
            write!(f, ",{}", i)?;
        }
        Ok(())
    }
}

impl<Item, Iter> From<Iter> for List<Iter>
where
    Item: ToString,
    for<'a> &'a Iter: IntoIterator<Item = &'a Item>,
{
    fn from(i: Iter) -> Self {
        Self(i)
    }
}

/// Major and minor versions of VK API
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Version(pub u8, pub u16);

impl Default for Version {
    fn default() -> Self {
        Self(5, 131)
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

#[cfg(test)]
mod tests {
    use crate::structs::{List, Version};

    #[test]
    fn serialize_ints() {
        assert_eq!(List(vec![1, 2, 3]).to_string(), "1,2,3".to_owned());
    }

    #[test]
    fn serialize_strs() {
        assert_eq!(
            List(vec!["id", "sex", "age"]).to_string(),
            "id,sex,age".to_owned()
        );
    }

    #[test]
    fn serialize_version() {
        assert_eq!(Version(5, 131).to_string(), "5.131".to_owned());
    }
}
