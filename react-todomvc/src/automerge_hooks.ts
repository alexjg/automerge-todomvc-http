import React from 'react';
import * as automerge from "automerge"
import type { Doc } from "automerge"
import type { TodoType } from "./todo"

// https://www.w3resource.com/javascript-exercises/javascript-math-exercise-23.php
function uuid() {
  var dt = new Date().getTime();
  var uuid = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(
    c
  ) {
    var r = (dt + Math.random() * 16) % 16 | 0;
    dt = Math.floor(dt / 16);
    return (c == 'x' ? r : (r & 0x3) | 0x8).toString(16);
  });
  return uuid;
}

type TodoApp = {
  todos: TodoType[]
}

function initDoc(): Doc<TodoApp>{
  let changeFn = (doc: TodoApp) => {
    doc.todos = []
  }
  let change = automerge.getLastLocalChange(
    automerge.change<Doc<TodoApp>>(automerge.init('0000'), { time: 0 }, changeFn)
  );
  let [doc, patch] = automerge.applyChanges<TodoApp>(
    automerge.init(), [change]
  )
  return doc
}


class AutomergeTodos {
  private _doc: Doc<TodoApp>

  constructor(doc: Doc<{todos: TodoType[]}>) {
    this._doc = doc
  }

  get todos(): TodoType[] {
    return this._doc.todos
  }

  async changeTodo(todoId: string, changeFn: (todo: TodoType) =>  void): Promise<AutomergeTodos> {
    const newDoc = automerge.change(this._doc, doc => {
      for (const todo of doc.todos) {
        if (todo.id === todoId){
          changeFn(todo)
        }
      }
    })
    return new AutomergeTodos(newDoc)
  }

  async addNewTodo(value: string): Promise<AutomergeTodos> {
    const newDoc = automerge.change(this._doc, doc => {
      doc.todos.push({
        id: uuid(),
        value,
        completed: false,
      })
    })
    return new AutomergeTodos(newDoc)
  }

  async clearCompletedTodos(): Promise<AutomergeTodos> {
    const newDoc = automerge.change(this._doc, doc => {
      let i = 0;
      let numTodos = doc.todos.length
      while (i < numTodos) {
          const todo = doc.todos[i]
          if (todo.completed) {
              delete doc.todos[i]
              numTodos -= 1
          } else {
              i++
          }
      }
    })
    return new AutomergeTodos(newDoc)
  }

  async applyChanges(doc: automerge.BinaryDocument): Promise<AutomergeTodos> {
    const otherDoc = automerge.load(doc)
    const otherChanges = automerge.getAllChanges(otherDoc)
    const [newDoc, patch] = automerge.applyChanges(this._doc, otherChanges)
    return new AutomergeTodos(newDoc)
  }

  getChanges(): Uint8Array {
    //const changes = automerge.getAllChanges(this._doc)
    //const flatNumberArray = changes.reduce<number[]>((acc, curr) => {
      //acc.push(...curr);
      //return acc
    //}, [])
    //return new Uint8Array(flatNumberArray)
    return automerge.save(this._doc)
  }
}

export function useAutomergeTodos(): AutomergeTodosHooks {
  const [todoDoc, setTodoDoc] = React.useState<AutomergeTodos>(new AutomergeTodos(initDoc()))
  return {
    todos: todoDoc.todos,
    updateTodo: async (newTodo: TodoType) => {
      let newDoc = await todoDoc.changeTodo(newTodo.id, (oldTodo) => {
        oldTodo.value = newTodo.value
        oldTodo.completed = newTodo.completed
      })
      setTodoDoc(newDoc)
    },
    addNewTodo: async (value: string) => {
      setTodoDoc(await todoDoc.addNewTodo(value))
    },
    clearCompletedTodos: async () => {
      setTodoDoc(await todoDoc.clearCompletedTodos())
    },
    loadRemoteTodos: async (url: string) => {
      const response = await fetch(url)
      const respbuffer = await response.arrayBuffer()
      const view = new Uint8Array(respbuffer)
      setTodoDoc(await todoDoc.applyChanges(view as automerge.BinaryDocument))
    },
    pushTodosToRemote: async (url: string) => {
      const changes = todoDoc.getChanges()
      const response = await fetch(url, {
        body: changes,
        method: "post",
        headers: {
          "Content-Type": "application/octet-stream",
        }
      })
      console.log(response)
    }
  }
}

export type AutomergeTodosHooks = {
  todos: TodoType[],
  updateTodo: (newTodo: TodoType) => Promise<void>,
  addNewTodo: (value: string) => Promise<void>,
  clearCompletedTodos: () => Promise<void>,
  loadRemoteTodos: (url: string) => Promise<void>,
  pushTodosToRemote: (url: string) => Promise<void>,
}
