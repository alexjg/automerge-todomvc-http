import flask
from flask_cors import CORS
import pathlib

app = flask.Flask("databoy")
CORS(app)

@app.route("/<path:filename>", methods=["GET"])
def get_file(filename):
    path: pathlib.Path = pathlib.Path("./data") / filename
    if not path.exists():
        return flask.make_response("not found", 404)
    return flask.send_from_directory("./data", filename)


@app.route("/<path:filename>", methods=["POST"])
def upload_file(filename):
    with open(f"./data/{filename}", "wb") as outfile:
        outfile.write(flask.request.stream.read())
    return "done"


if __name__ == "__main__":
    app.run(debug=True)


