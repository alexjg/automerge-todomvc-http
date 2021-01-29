import * as React from 'react';
import { Todos } from "./Todos";
import { useAutomergeTodos, AutomergeTodosHooks } from "./automerge_hooks"
import { Peers } from "./Peers"

export const App = () => {
  const props: AutomergeTodosHooks = useAutomergeTodos();
  return (
    <div>
      <Peers onFetchUrl={props.loadRemoteTodos} onPushToUrl={props.pushTodosToRemote}/>
      <Todos {...props}>
      </Todos>
    </div>
  );
};
