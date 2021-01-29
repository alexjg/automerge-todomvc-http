# react-todomvc

This is a heavily modified fork of [github.com/sw-yx/react-todomvc](https://github.com/sw-yx/react-todomvc#readme). It depends on the `performance` branch of automerge which means that you will need to do a bit of manual work to get it up and running:

```bash
npm install
cd node_modules/automerge && npm install && npm build
```

Once this is done you can do `npm start` and then go to `http://localhost:1234` to see the application.
