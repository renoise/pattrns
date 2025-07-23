use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use mlua::prelude::*;

// -------------------------------------------------------------------------------------------------

// Limits script execution time and aborts execution when a script runs too long. This way e.g.
// never ending loops are stopped automatically with a timeout error.
//
// While constructed, it checks every few instructions if a timeout duration has been reached
// and then aborts the script by firing an error.
// When cloning and instance, it will use the existing hook, so ensure to call `reset` before
// invoking new lua functions. The last instance that get's dropped will then remove the hook.
#[derive(Debug)]
pub(crate) struct LuaTimeoutHook {
    active: Rc<RefCell<usize>>,
    start: Rc<RefCell<Instant>>,
}

impl LuaTimeoutHook {
    // default number of ms a script may run before a timeout error is fired.
    // assumes scripts are running in a real-time alike context.
    const DEFAULT_TIMEOUT: Duration = Duration::from_millis(200);

    pub(crate) fn new(lua: &Lua) -> Self {
        Self::new_with_timeout(lua, Self::DEFAULT_TIMEOUT)
    }

    pub(crate) fn new_with_timeout(lua: &Lua, timeout: Duration) -> Self {
        let active = Rc::new(RefCell::new(1));
        let start = Rc::new(RefCell::new(Instant::now()));
        let timeout_hook = {
            let active = Rc::clone(&active);
            let start = Rc::clone(&start);
            move || {
                if *active.borrow() > 0 {
                    if start.borrow().elapsed() > timeout {
                        *start.borrow_mut() = Instant::now();
                        Err(LuaError::RuntimeError(
                            String::from("Script timeout. ")
                                + &format!("Execution took longer than {} ms to complete.\n", timeout.as_millis())
                                + "Please avoid overhead and check for never ending loops in your script. "
                                + "Also note that the script is running in real-time thread!",
                        ))
                    } else {
                        Ok(false) // continue running
                    }
                } else {
                    Ok(true) // remove hook
                }
            }
        };
        // Lua or LuaJij -> set_hook
        #[cfg(not(any(feature = "luau", feature = "luau-jit")))]
        {
            lua.set_hook(
                LuaHookTriggers::new().every_nth_instruction(timeout.as_millis() as u32 * 10),
                move |lua, _debug| match timeout_hook() {
                    Ok(remove_hook) => {
                        if remove_hook {
                            lua.remove_hook();
                        }
                        Ok(LuaVmState::Continue)
                    }
                    Err(err) => Err(err),
                },
            )
            .expect("Failed to install lua hook");
        }
        // Luau -> set_interrupt
        #[cfg(any(feature = "luau", feature = "luau-jit"))]
        {
            lua.set_interrupt(move |lua| match timeout_hook() {
                Ok(remove_hook) => {
                    if remove_hook {
                        lua.remove_interrupt();
                    }
                    Ok(LuaVmState::Continue)
                }
                Err(err) => Err(err),
            });
        }
        Self { active, start }
    }

    // reset timestamp of the hook when running e.g. a callback again
    pub(crate) fn reset(&mut self) {
        *self.start.borrow_mut() = Instant::now();
    }
}

impl Clone for LuaTimeoutHook {
    fn clone(&self) -> Self {
        // increase active instances refcount
        *self.active.borrow_mut() += 1;
        // return a direct clone otherwise
        Self {
            active: Rc::clone(&self.active),
            start: Rc::clone(&self.start),
        }
    }
}

impl Drop for LuaTimeoutHook {
    fn drop(&mut self) {
        // decrease active instance refcount.
        *self.active.borrow_mut() -= 1;
        // when reaching 0, this will remove the hook in the hook itself
    }
}
