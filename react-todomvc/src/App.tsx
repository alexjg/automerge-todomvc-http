import * as React from 'react';
import { Todos } from "./Todos";
import { useAutomergeTodos, AutomergeTodosHooks } from "./automerge_hooks"
import { Remotes } from "./Remotes"

export const App = () => {
  const props: AutomergeTodosHooks = useAutomergeTodos();
  return (
    <div className="container">
      <Todos {...props}>
      </Todos>
      <Remotes onFetchUrl={props.loadRemoteTodos} onPushToUrl={props.pushTodosToRemote}/>
    </div>
  );
};
