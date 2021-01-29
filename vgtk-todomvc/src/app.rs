use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::convert::TryInto;
use std::str::FromStr;

use vgtk::lib::gio::{ActionExt, ApplicationFlags, File, FileExt, SimpleAction};
use vgtk::lib::glib::Error;
use vgtk::lib::gtk::prelude::*;
use vgtk::lib::gtk::*;
use vgtk::{ext::*, gtk, gtk_if, on_signal, Component, UpdateAction, VNode};

use strum_macros::{Display, EnumIter};

use crate::about::AboutDialog;
use crate::items::{Item, Items};
use crate::radio::Radio;

use maplit::hashmap;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Display, EnumIter)]
pub enum Filter {
    All,
    Active,
    Completed,
}

impl Default for Filter {
    fn default() -> Self {
        Filter::All
    }
}

#[derive(Clone, Debug)]
pub struct Peer {
    url: String,
}

impl Peer {
    fn render(&self, index: usize) -> VNode<Model> {
        let url = self.url.clone();
        gtk!{
            <Box spacing=10 orientation=Orientation::Horizontal>
                <Label label=self.url.to_string() />
                <Button label="pull" on clicked=|_| { Msg::PullFromPeer { peer_index: index }}/>
                <Button label="push" on clicked=|_| { Msg::PushToPeer { peer_index: index }}/>
            </Box>
        }
    }
}

#[derive(Clone)]
pub struct Model {
    filter: Filter,
    file: Option<File>,
    clean: bool,
    peers: Vec<Peer>,
    backend: Arc<Mutex<automerge::Backend>>,
    frontend: Arc<Mutex<automerge::Frontend>>,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Model(items: {:?}, filter: {:?}, file: {:?}, clean: {:?}, peers: {:?})", self.items(), self.filter, self.file, self.clean, self.peers)
    }
}

impl Default for Model {
    fn default() -> Self {
        let mut backend = automerge::Backend::init();
        let change = automerge::Change::from_bytes(crate::INIT_CHANGE.to_vec()).unwrap();
        let patch = backend.apply_changes(vec![change]).unwrap();
        let mut frontend = automerge::Frontend::new();
        frontend.apply_patch(patch).unwrap();
        Model {
            filter: Filter::All,
            file: None,
            clean: true,
            peers: Vec::new(),
            backend: Arc::new(Mutex::new(backend)),
            frontend: Arc::new(Mutex::new(frontend)),
        }
    }
}

impl Model {
    fn filter(&self, filter: Filter) -> Vec<Item> {
        let items = self.items();
        items.iter().filter(move |item| match filter {
            Filter::All => true,
            Filter::Active => !item.done,
            Filter::Completed => item.done,
        }).cloned().collect::<Vec<Item>>()
    }

    fn items(&self) -> Items {
        let mut frontend = self.frontend.lock().unwrap();
        let state = frontend.state();
        state.try_into().unwrap()
    }

    fn left_label(&self) -> String {
        let left = self.filter(Filter::Active).iter().count();
        match left {
            1 => String::from("1 item left"),
            left => format!("{} items left", left),
        }
    }

    fn main_panel(&self) -> VNode<Model> {
        gtk! {
            <Box orientation=Orientation::Horizontal>
                <Box spacing=10 orientation=Orientation::Vertical Box::fill=true Box::expand=true>
                    <Box spacing=10 orientation=Orientation::Horizontal Box::expand=false>
                        <Button image="edit-select-all" relief=ReliefStyle::Half
                                always_show_image=true on clicked=|_| Msg::ToggleAll/>
                        <Entry placeholder_text="What needs to be done?"
                               Box::expand=true Box::fill=true
                               on activate=|entry| {
                                   let label = entry.get_text().to_string();
                                   entry.select_region(0, label.len() as i32);
                                   Msg::Add {
                                       item: label
                                   }
                               } />
                    </Box>
                    <ScrolledWindow Box::expand=true Box::fill=true>
                        <ListBox selection_mode=SelectionMode::None>
                            {
                                self.filter(self.filter).iter().enumerate()
                                    .map(|(index, item)| item.render(index))
                            }
                        </ListBox>
                    </ScrolledWindow>
                    <Box spacing=10 orientation=Orientation::Horizontal Box::expand=false>
                        <Label label=self.left_label()/>
                        <@Radio<Filter> active=self.filter Box::center_widget=true on changed=|filter| Msg::Filter { filter } />
                        {
                            gtk_if!(self.filter(Filter::Completed).iter().count() > 0 => {
                                <Button label="Clear completed" Box::pack_type=PackType::End
                                        on clicked=|_| Msg::ClearCompleted/>
                            })
                        }
                    </Box>
                </Box>
                <Box spacing=10 orientation=Orientation::Vertical>
                    <Label label="Peers" width_chars=50/>  
                    <Box spacing=10 orientation=Orientation::Horizontal Box::expand=false>
                        <Entry placeholder_text="Peer url"
                            Box::expand=true 
                            on activate=|entry| {
                                let label = entry.get_text().to_string();
                                entry.select_region(0, label.len() as i32);
                                Msg::AddPeer {
                                    url: label
                                }
                            } />
                    </Box>
                    {self.peers.iter().enumerate().map(|(index, peer)| peer.render(index))}
                </Box>
            </Box>
        }
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    NoOp,
    Add { item: String },
    Remove { index: usize },
    Toggle { index: usize },
    Filter { filter: Filter },
    ToggleAll,
    ClearCompleted,
    Exit,
    MenuAbout,
    AddPeer { url:  String },
    PullFromPeer { peer_index: usize},
    PushToPeer { peer_index: usize},
    PullComplete,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn update(&mut self, msg: Self::Message) -> UpdateAction<Self> {
        let left = self.filter(Filter::Active).iter().count();
        match msg {
            Msg::NoOp => return UpdateAction::None,
            Msg::Add { item } => {
                let new_item = Item::new(item.clone());
                let mut frontend = self.frontend.lock().unwrap();
                let change = frontend.change::<_, automerge::InvalidChangeRequest>(Some("Add item".to_string()), |doc| {
                    let todos = doc.value_at_path(&automerge::Path::root().key("todos")).unwrap();
                    let num_todos = match todos {
                        automerge::Value::Sequence(elems) => elems.len(),
                        _ => panic!()
                    };
                    doc.add_change(automerge::LocalChange::insert(
                        automerge::Path::root().key("todos").index(num_todos.try_into().unwrap()),
                        hashmap!{
                            "value" => automerge::Value::Primitive(new_item.task.as_str().into()),
                            "completed" => automerge::Value::Primitive(new_item.done.into()),
                            "id" => automerge::Value::Primitive(new_item.id.as_str().into()),
                        }.into()
                    ))?;
                    Ok(())
                }).unwrap();
                if let Some(change) = change {
                    let mut backend = self.backend.lock().unwrap();
                    let patch = backend.apply_local_change(change).unwrap().0;
                    frontend.apply_patch(patch).unwrap();
                }
                self.clean = false;
            }
            Msg::Remove { index } => {
                //Arc::make_mut(&mut self.items).remove(index);
                let mut frontend = self.frontend.lock().unwrap();
                let change = frontend.change::<_, automerge::InvalidChangeRequest>(None, |doc| {
                    doc.add_change(
                        automerge::LocalChange::delete(automerge::Path::root().key("todos").index(index.try_into().unwrap()))
                    )?;
                    Ok(())
                }).unwrap();
                let mut backend = self.backend.lock().unwrap();
                if let Some(change) = change {
                    let patch = backend.apply_local_change(change).unwrap().0;
                    frontend.apply_patch(patch).unwrap();
                }
                self.clean = false;
            }
            Msg::Toggle { index } => {
                let mut frontend = self.frontend.lock().unwrap();
                let change = frontend.change::<_, automerge::InvalidChangeRequest>(None, |doc| {
                    let index: u32 = index.try_into().unwrap();
                    let existing = doc.value_at_path(&automerge::Path::root().key("todos").index(index).key("completed"));
                    let current = match existing {
                        Some(automerge::Value::Primitive(automerge::ScalarValue::Boolean(b))) => b,
                        _ => panic!()
                    };
                    doc.add_change(automerge::LocalChange::set(
                        automerge::Path::root().key("todos").index(index).key("completed"),
                        automerge::ScalarValue::Boolean(!current).into(),
                    ))?;
                    Ok(())
                }).unwrap();
                if let Some(change) = change {
                    let mut backend = self.backend.lock().unwrap();
                    let patch = backend.apply_local_change(change).unwrap().0;
                    frontend.apply_patch(patch).unwrap();
                }
                self.clean = false;
            }
            Msg::Filter { filter } => {
                self.filter = filter;
            }
            Msg::ToggleAll if left > 0 => {
                let filtered_ids: Vec<String> = self.filter(self.filter).iter().map(|i| i.id.clone()).collect();

                let mut frontend = self.frontend.lock().unwrap();
                let change = frontend.change::<_, automerge::InvalidChangeRequest>(None, |doc| {
                    let todos = doc.value_at_path(&automerge::Path::root().key("todos"));
                    let todos_count: u32 = match todos {
                        Some(automerge::Value::Sequence(elems)) => elems.len() as u32,
                        _ => panic!()
                    };
                    for i in 0..todos_count {
                        let path = automerge::Path::root().key("todos").index(i);
                        let current_value = match doc.value_at_path(&path) {
                            Some(automerge::Value::Primitive(automerge::ScalarValue::Boolean(b))) => b,
                            _ => panic!()
                        };
                        let id_path = automerge::Path::root().key("todos").index(i).key("id");
                        let current_id = match doc.value_at_path(&id_path) {
                            Some(automerge::Value::Primitive(automerge::ScalarValue::Str(id))) => id,
                            _ => panic!(),
                        };
                        if filtered_ids.contains(&current_id) {
                            doc.add_change(automerge::LocalChange::set(path, automerge::Value::Primitive(automerge::ScalarValue::Boolean(!current_value))))?;
                        }
                    }
                    Ok(())
                }).unwrap();
                if let Some(change) = change {
                    let mut backend = self.backend.lock().unwrap();
                    let patch = backend.apply_local_change(change).unwrap().0;
                    frontend.apply_patch(patch).unwrap()
                }
                self.clean = false;
            }
            Msg::ToggleAll => return UpdateAction::None,
            Msg::ClearCompleted => {
                let mut frontend = self.frontend.lock().unwrap();
                let change = frontend.change::<_, automerge::InvalidChangeRequest>(None, |doc| {
                    let todos = doc.value_at_path(&automerge::Path::root().key("todos"));
                    let mut todos_count: u32 = match todos {
                        Some(automerge::Value::Sequence(elems)) => elems.len() as u32,
                        _ => panic!()
                    };
                    let mut i = 0;
                    while i < todos_count {
                        let path = automerge::Path::root().key("todos").index(i);
                        let current_value = match doc.value_at_path(&path.clone().key("completed")) {
                            Some(automerge::Value::Primitive(automerge::ScalarValue::Boolean(b))) => b,
                            _ => panic!()
                        };
                        if current_value {
                            todos_count -= 1;
                            doc.add_change(automerge::LocalChange::delete(path))?;
                        } else {
                            i += 1;
                        }
                    }
                    Ok(())
                }).unwrap();
                if let Some(change) = change {
                    let mut backend = self.backend.lock().unwrap();
                    let patch = backend.apply_local_change(change).unwrap().0;
                    frontend.apply_patch(patch).unwrap()
                }
                self.clean = false;
                return UpdateAction::Render
            }
            Msg::Exit => {
                vgtk::quit();
                return UpdateAction::None;
            }
            Msg::MenuAbout => {
                AboutDialog::run();
                return UpdateAction::None;
            }
            Msg::AddPeer { url } => {
                self.peers.push(Peer{url});
            }
            Msg::PullFromPeer { peer_index } => {
                let peer = self.peers.get(peer_index).unwrap();
                let url = peer.url.clone();
                let frontend = self.frontend.clone();
                let backend = self.backend.clone();
                let url: reqwest::Url = reqwest::Url::from_str(url.as_str()).unwrap();
                return UpdateAction::defer(async move {
                    let bytes = reqwest::blocking::get(url).and_then(|r| r.bytes()).unwrap();
                    let changes: Vec<automerge::Change> = automerge::Change::load_document(&bytes).unwrap();
                    let mut backend = backend.lock().unwrap();
                    let patch = backend.apply_changes(changes).unwrap();
                    let mut frontend = frontend.lock().unwrap();
                    frontend.apply_patch(patch).unwrap();
                    Msg::PullComplete
                })
            }
            Msg::PushToPeer{ peer_index } => {
                let peer = self.peers.get(peer_index).unwrap();
                let backend = self.backend.lock().unwrap();
                let raw = backend.save().unwrap();
                let client = reqwest::blocking::Client::new();
                let url = reqwest::Url::from_str(peer.url.as_str()).unwrap();
                client.post(url).body(raw).send().unwrap();
            }
            Msg::PullComplete => return UpdateAction::Render
        }
        UpdateAction::Render
    }

    fn view(&self) -> VNode<Model> {
        let title = if let Some(name) = self.file.as_ref().and_then(|p| p.get_basename()) {
            name.to_str().unwrap().to_string()
        } else {
            "Untitled todo list".to_string()
        };
        let clean = if self.clean { "" } else { " *" };

        use vgtk::menu;
        let main_menu = menu()
            .section(menu().item("Open...", "win.open"))
            .section(
                menu()
                    .item("Save", "win.save")
                    .item("Save as...", "win.save-as"),
            )
            .section(menu().item("About...", "app.about"))
            .section(menu().item("Quit", "app.quit"))
            .build();

        gtk! {
            <Application::new_unwrap(Some("camp.lol.todomvc"), ApplicationFlags::empty())>

                <SimpleAction::new("quit", None) Application::accels=["<Ctrl>q"].as_ref() enabled=true
                        on activate=|a, _| Msg::Exit/>
                <SimpleAction::new("about", None) enabled=true on activate=|_, _| Msg::MenuAbout/>

                <ApplicationWindow default_width=1200 default_height=480 border_width=20 on destroy=|_| Msg::Exit>

                    <HeaderBar title=format!("TodoMVC - {}{}", title, clean) subtitle="wtf do we do now" show_close_button=true>
                        <MenuButton HeaderBar::pack_type=PackType::End @MenuButtonExt::direction=ArrowType::Down relief=ReliefStyle::None
                                    image="open-menu-symbolic">
                            <Menu::from_model(&main_menu)/>
                        </MenuButton>
                    </HeaderBar>
                    {
                        self.main_panel()
                    }
                </ApplicationWindow>
            </Application>
        }
    }
}

async fn open() -> Result<Option<(File, Items)>, Error> {
    let dialog = FileChooserNative::new(
        Some("Open a todo list"),
        vgtk::current_object()
            .and_then(|w| w.downcast::<Window>().ok())
            .as_ref(),
        FileChooserAction::Open,
        None,
        None,
    );
    dialog.set_modal(true);
    let filter = FileFilter::new();
    filter.set_name(Some("Todo list files"));
    filter.add_pattern("*.todo");
    dialog.add_filter(&filter);
    dialog.show();
    if on_signal!(dialog, connect_response).await == Ok(ResponseType::Accept) {
        let file = dialog.get_file().unwrap();
        Items::read_from(&file)
            .await
            .map(|items| Some((file, items)))
    } else {
        Ok(None)
    }
}

async fn save(items: &Items, file: &File) -> Result<(), Error> {
    items.write_to(file).await
}

async fn save_as(items: &Items) -> Result<Option<File>, Error> {
    let dialog = FileChooserNative::new(
        Some("Save your todo list"),
        vgtk::current_window().as_ref(),
        FileChooserAction::Save,
        None,
        None,
    );
    dialog.set_modal(true);
    let filter = FileFilter::new();
    filter.set_name(Some("Todo list files"));
    filter.add_pattern("*.todo");
    dialog.add_filter(&filter);
    dialog.show();
    if on_signal!(dialog, connect_response).await == Ok(ResponseType::Accept) {
        let file = dialog.get_file().unwrap();
        save(items, &file).await.map(|_| Some(file))
    } else {
        Ok(None)
    }
}
