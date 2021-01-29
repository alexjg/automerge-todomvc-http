use std::convert::{TryFrom, TryInto};
use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};

use vgtk::lib::gio::{File, FileCreateFlags, FileExt, OutputStreamExt};
use vgtk::lib::glib::{Bytes, Error, FileError};
use vgtk::lib::gtk::*;

use vgtk::{gtk, VNode};

use serde_derive::{Deserialize, Serialize};

use automerge::{ScalarValue, Value};

use crate::app::{Model, Msg};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Item {
    pub task: String,
    pub id: String,
    pub done: bool,
}

impl Item {
    pub fn new<S: Into<String>>(label: S) -> Self {
        Item {
            task: label.into(),
            id: uuid::Uuid::new_v4().to_string(),
            done: false,
        }
    }

    pub fn render(&self, index: usize) -> VNode<Model> {
        let label = if self.done {
            format!(
                "<span strikethrough=\"true\" alpha=\"50%\">{}</span>",
                self.task
            )
        } else {
            self.task.clone()
        };
        gtk! {
            <ListBoxRow>
                <Box spacing=10 orientation=Orientation::Horizontal>
                    <CheckButton active=self.done on toggled=|_| Msg::Toggle { index } />
                    <Label label=label use_markup=true Box::fill=true />
                    <Button Box::pack_type=PackType::End relief=ReliefStyle::None
                            always_show_image=true image="edit-delete"
                            on clicked=|_| Msg::Remove { index } />
                </Box>
            </ListBoxRow>
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct Items {
    items: Vec<Item>,
}

impl Items {
    pub async fn read_from(file: &File) -> Result<Items, Error> {
        serde_json::from_slice(&file.load_contents_async_future().await?.0)
            .map(|items| Items { items })
            .map_err(|err| {
                Error::new(
                    FileError::Inval,
                    &format!(
                        "Parse error in file \"{}\": {}",
                        file.get_basename().unwrap().to_str().unwrap(),
                        err
                    ),
                )
            })
    }

    pub async fn write_to(&self, file: &File) -> Result<(), Error> {
        let data = serde_json::to_vec_pretty(&self.items)
            .map_err(|err| Error::new(FileError::Inval, &format!("{}", err)))?;
        let out = file
            .replace_async_future(None, false, FileCreateFlags::empty(), Default::default())
            .await?;
        out.write_bytes_async_future(&Bytes::from_owned(data), Default::default())
            .await?;
        out.close_async_future(Default::default()).await
    }
}

impl Deref for Items {
    type Target = Vec<Item>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl DerefMut for Items {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl FromIterator<Item> for Items {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Item>,
    {
        Items {
            items: iter.into_iter().collect(),
        }
    }
}

impl TryFrom<&automerge::Value> for Item {
    type Error = String;

    fn try_from(value: &automerge::Value) -> Result<Self, Self::Error> {
        match value {
            automerge::Value::Map(values, automerge::MapType::Map) => {
                let id = values.get("id");
                let task = values.get("value");
                let completed = values.get("completed");
                match (id, task, completed) {
                    (
                        Some(Value::Primitive(ScalarValue::Str(id))),
                        Some(Value::Primitive(ScalarValue::Str(task))),
                        Some(Value::Primitive(ScalarValue::Boolean(done))),
                    ) => Ok(Item {
                        id: id.to_string(),
                        task: task.to_string(),
                        done: *done,
                    }),
                    _ => Err("invalid value for item".to_string()),
                }
            }
            _ => Err("attempted to create an item from a non-map type".to_string()),
        }
    }
}

impl TryFrom<&automerge::Value> for Items {
    type Error = String;

    fn try_from(value: &automerge::Value) -> Result<Self, Self::Error> {
        match value {
            automerge::Value::Map(items, automerge::MapType::Map) => {
                let todos = items.get("todos").ok_or("No 'todos' key found")?;
                match todos {
                    automerge::Value::Sequence(elems) => {
                        let init: Vec<Item> = Vec::new();
                        let items = elems.iter().try_fold(init, |mut items, value| -> Result<Vec<Item>, String> {
                            let item: Item = value.try_into()?;
                            items.push(item);
                            Ok(items)
                        })?;
                        Ok(Items{items})
                    },
                    _ => Err("todos key did not contain a sequence".to_string())
                }
            }
            _ => Err("attempted to create items from something which wasn't a map".to_string())
        }
    }
}
