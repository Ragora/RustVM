#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use std::{time::{Instant}, collections::{HashMap}, hash::{SipHasher}};
    use std::sync::{RwLock, Arc};
    use std::thread;
    use std::sync;

    use crate::util::variable_name_to_identifier;
    use crate::vm::{InstructionSequence, OpCode, VariableReference, Function, RawValue, VariableIdentifier, PushFloat, AddressValue, VirtualMachine};

    #[test]
    fn test_functions()
    {
        // FIXME: Get a more accurate compile result by sourcing this post-compile
        let opcodes = InstructionSequence { 
            ops: vec![
                OpCode::CallFunction { target: vec!["quit".to_owned()] },
            ]
        };

        let vm = VirtualMachine::new();

        // Add a native binding
        let mut namespace_write = vm.root_namespace.borrow_mut();
        namespace_write.add_function_entry(Function::NativeFunction { 
            parameters: Vec::new(), 
            binding: |_vm, _frame| -> Result<(), &'static str> {
                Ok(())
            }
        }, &vec!["quit".to_owned()]).unwrap();
        drop(namespace_write);

        // Run code
        let start_time = Instant::now();
        for _ in 0 .. 99999
        {
            vm.interpret(&opcodes).unwrap();   
        }
        let end_time = Instant::now();
        let delta = end_time - start_time;

        println!("0param Call Time: {:?}", delta);
    }

    #[test]
    fn test_vm() {
          // FIXME: Get a more accurate compile result by sourcing this post-compile
        let opcodes_a = InstructionSequence { 
            ops: vec![
            // Assign %counter = 0
            OpCode::PushInteger { value: 0 },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_a".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },
            
            // Assign %result = 0.0
            OpCode::PushFloat { 0: PushFloat { value: 0.0 }},
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_a".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // Assign %iterations
            OpCode::PushInteger { value: 999999 },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("iterations_a".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // 12th index is start of program
            OpCode::NOP { },

            // Loop %iterations iterations with current VM state and perform a calculation
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_a".to_owned()) } },
            OpCode::PushFloat { 0: PushFloat { value: 3.14 }},
            OpCode::Add { },

            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_a".to_owned()) } },
            OpCode::Swap { }, // Crappy workaround until I feel like fixing the stack arrangement
            OpCode::Assignment { },
            OpCode::Pop { },

            // Increment counter
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_a".to_owned()) } },
            OpCode::PushInteger { value: 1 },
            OpCode::Add { },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_a".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // Check if loop condition is met - %counter >= %iterations
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("iterations_a".to_owned()) } },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_a".to_owned()) } },

            OpCode::GreaterThanOrEqual { },
            OpCode::JumpFalse { target: AddressValue::AbsoluteTarget { index: 12 } },

            // Write final result to a global
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_a".to_owned()) } },
            OpCode::PushVariable { variable: VariableReference::Global { value: variable_name_to_identifier("result_a".to_owned()) } },
            OpCode::Assignment {  }
        ]};

        let opcodes_b = InstructionSequence { 
            ops: vec![
            // Assign %counter = 0
            OpCode::PushInteger { value: 0 },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_b".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },
            
            // Assign %result = 0.0
            OpCode::PushFloat { 0: PushFloat { value: 0.0 }},
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_b".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // Assign %iterations
            OpCode::PushInteger { value: 999999 },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("iterations_b".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // 12th index is start of program
            OpCode::NOP { },

            // Loop %iterations iterations with current VM state and perform a calculation
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_b".to_owned()) } },
            OpCode::PushFloat { 0: PushFloat { value: 3.14 }},
            OpCode::Add { },

            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_b".to_owned()) } },
            OpCode::Swap { }, // Crappy workaround until I feel like fixing the stack arrangement
            OpCode::Assignment { },
            OpCode::Pop { },

            // Increment counter
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_b".to_owned()) } },
            OpCode::PushInteger { value: 1 },
            OpCode::Add { },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_b".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // Check if loop condition is met - %counter >= %iterations
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("iterations_b".to_owned()) } },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_b".to_owned()) } },

            OpCode::GreaterThanOrEqual { },
            OpCode::JumpFalse { target: AddressValue::AbsoluteTarget { index: 12 } },

            // Write final result to a global
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_b".to_owned()) } },
            OpCode::PushVariable { variable: VariableReference::Global { value: variable_name_to_identifier("result_b".to_owned()) } },
            OpCode::Assignment {  }
        ]};

        let opcodes_c = InstructionSequence { 
            ops: vec![
            // Assign %counter = 0
            OpCode::PushInteger { value: 0 },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_c".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },
            
            // Assign %result = 0.0
            OpCode::PushFloat { 0: PushFloat { value: 0.0 }},
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_c".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // Assign %iterations
            OpCode::PushInteger { value: 999999 },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("iterations_c".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // 12th index is start of program
            OpCode::NOP { },

            // Loop %iterations iterations with current VM state and perform a calculation
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_c".to_owned()) } },
            OpCode::PushFloat { 0: PushFloat { value: 3.14 }},
            OpCode::Add { },

            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_c".to_owned()) } },
            OpCode::Swap { }, // Crappy workaround until I feel like fixing the stack arrangement
            OpCode::Assignment { },
            OpCode::Pop { },

            // Increment counter
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_c".to_owned()) } },
            OpCode::PushInteger { value: 1 },
            OpCode::Add { },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_c".to_owned()) } },
            OpCode::Assignment { },
            OpCode::Pop { },

            // Check if loop condition is met - %counter >= %iterations
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("iterations_c".to_owned()) } },
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("counter_c".to_owned()) } },

            OpCode::GreaterThanOrEqual { },
            OpCode::JumpFalse { target: AddressValue::AbsoluteTarget { index: 12 } },

            // Write final result to a global
            OpCode::PushVariable { variable: VariableReference::Local { value: variable_name_to_identifier("result_c".to_owned()) } },
            OpCode::PushVariable { variable: VariableReference::Global { value: variable_name_to_identifier("result_c".to_owned()) } },
            OpCode::Assignment {  }
        ]};

        // Ask VM to execute top level machine code
        let vm = VirtualMachine::new();
        #[cfg(feature="async")]
        {
            let start_time = Instant::now();

            // Wrap the VM in an Arc
            let vm_handle = Arc::new(vm);

            let vm_a_handle = vm_handle.clone();
            let vm_b_handle = vm_handle.clone();
            let vm_c_handle = vm_handle.clone();

            let thread_a = thread::spawn(move || {
                vm_a_handle.interpret(&opcodes_a).unwrap();
            });
            
            let thread_b = thread::spawn(move || {
                vm_b_handle.interpret(&opcodes_b).unwrap();
            });

            let thread_c = thread::spawn(move || {
                vm_c_handle.interpret(&opcodes_c).unwrap();
            });
            thread_a.join().unwrap();
            thread_b.join().unwrap();
            thread_c.join().unwrap();
            
            let end_time = Instant::now();
            let delta = end_time - start_time;
        
            let globals_read: sync::RwLockReadGuard<HashMap<VariableIdentifier, RawValue>> = vm_handle.globals.read().unwrap();
            println!("{:?} {:?} {:?} Exec Time: {:?}", globals_read.get(&variable_name_to_identifier("result_a".to_owned())).unwrap(), 
            globals_read.get(&variable_name_to_identifier("result_b".to_owned())), 
            globals_read.get(&variable_name_to_identifier("result_c".to_owned())).unwrap(), delta);
        }

        #[cfg(not(feature="async"))]
        {
            let start_time = Instant::now();
        // for _ in 0 .. 2
            {
                vm.interpret(&opcodes_a).unwrap();
            }
            let end_time = Instant::now();
        
            let delta = end_time - start_time;
            let globals_read = &vm.globals.take();
            println!("{:?} Exec Time: {:?}", globals_read.get(&variable_name_to_identifier("result_a".to_owned())).unwrap(), delta);       
        }  
    }
}