# self { #self }
> A type that represents an instance that you call a function on. When you see a function signature starting with this type, you should use `:` to call the function on the instance, this way you can omit this first argument.
> ```lua
> local object = SomeClass()
> object:do_something(123)
> ```