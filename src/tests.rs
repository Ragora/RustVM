#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use std::{time::{Instant}, collections::{HashMap}, hash::{SipHasher}, cell::RefCell};
    use std::sync::{RwLock, Arc};
    use std::thread;
    use std::sync;

    use crate::util::variable_name_to_identifier;
    use crate::vm::{InstructionSequence, OpCode, VariableReference, Function, RawValue, VariableIdentifier, PushFloat, AddressValue, VirtualMachine};

    #[derive(Clone)]
    struct ApplicationState
    {
        pub running: bool
    }

    #[test]
    fn test_function_binding_simple()
    {  
        let opcodes = InstructionSequence { 
            ops: vec![
                // FIXME: Encode this ahead of time to avoid the CPU
                OpCode::CallFunction { target: vec!["quit".to_owned()] },
            ]
        };


        let vm = VirtualMachine::new(RefCell::new(ApplicationState {
            running: true
        }));

        // Add a native binding

        let mut namespace_write = vm.root_namespace.borrow_mut();
        namespace_write.add_function_entry(Function::NativeFunction { 
            parameters: Vec::new(), 
            binding: Box::new(|binding_vm, _frame| -> Result<(), &'static str> {
                let mut state_write = binding_vm.state.borrow_mut();
                state_write.running = false;
                drop(state_write);

                Ok(())
            })
        }, &vec!["quit".to_owned()]).unwrap();
        drop(namespace_write);

        // Perform execution
        vm.interpret(&opcodes).unwrap();   

        let state_read = vm.state.borrow();
        assert!(!state_read.running);
    }
}