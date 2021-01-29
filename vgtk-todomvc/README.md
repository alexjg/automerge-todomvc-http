# GTK todos

This is a GTK application which uses the Automerge rust library to add multiplayer abilities to Bodil Stokke's TODO MVC application written using her `vgtk` library, which is somewhat like React for Rust.

This code is copied from one of [the vgtk examples](https://github.com/bodil/vgtk/tree/master/examples/todomvc) and butchered to work with Automerge and simple HTTP sync. This is a demo so there is no error handling and in general things have been shoehorned into place.

`cargo run` will present you with a TODO MVC application which will allow you to specify http peers as with the react application
