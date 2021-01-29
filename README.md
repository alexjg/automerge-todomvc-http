# Simple HTTP Sync Demo

This is an interop demo for automerge. It's a simple todo list application running on as many platforms as automerge supports.

## HTTP Syncing

Automerge is agnostic as to how exactly peers find each other and communicate changes. We want to avoid getting into the minefield of peer to peer networking here, so we use a very simple HTTP sync method, but you can imagine this application built using libp2p.

What does HTTP syncing look like then? Well every implementation will make it's version of the document available at a publicly accessible HTTP endpoint. Each implementation provides a way for the user to pull changes from a particular URL into their document.

For the purposes of this demo there is a super simple flask application in `./server/server.py` which runs at `localhost:5000` and stores whatever you POST to it, e.g if you hit `POST http://localhost:5000/somefile` then `GET http://localhost:5000/somefile` will return the contents of that file.

## Schema

Every implementation in this application is syncing a document that is expected to look like this:

```json
{
    "todos": [
        {
            "value": "<some descriptive string>",
            "completed": false,
            "id": "<some string>"
        }
    ]
}
```

## Walkthrough

There is a javascript todo list implementation in `react-todomvc` and a Rust GTK application (only tested on linux) in `vgtk-todomvc`. Refer to each of those repositories for instructions on running them. We will also need the `automerge` CLI installed, which can be done with `cargo install --git https://github.com/automerge/automerge-rs --rev 24dcd9c1e646232850a9a422a9598ff18c350734` (provided you have [setup](https://doc.rust-lang.org/book/ch14-04-installing-binaries.html) `cargo install` to put binaries on your path).

You'll also need the server running, refer to `./server/README.md` for details.

So, let's assume we now have our simple HTTP sync server running and have a play.

To be continued ...

