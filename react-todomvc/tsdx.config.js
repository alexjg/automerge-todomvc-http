let css = require("rollup-plugin-css-only")

module.exports = {
  rollup(config) {
    config.plugins.push(css({
        output: "react-todomvc.css",
    }))
    return config;
  },
};
