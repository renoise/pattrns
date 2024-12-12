
---------------------------------------------------------
----------------Auto generated code block----------------
---------------------------------------------------------

do
    local searchers = package.searchers or package.loaders
    local origin_seacher = searchers[2]
    searchers[2] = function(path)
        local files =
        {
------------------------
-- Modules part begin --
------------------------

["erde.lib"] = function()
--------------------
-- Module: 'erde.lib'
--------------------
local _MODULE = {}
local compile = require("erde.compile")
local config = require("erde.config")
local PATH_SEPARATOR, VALID_LUA_TARGETS
do
	local __ERDE_TMP_6__
	__ERDE_TMP_6__ = require("erde.constants")
	PATH_SEPARATOR = __ERDE_TMP_6__["PATH_SEPARATOR"]
	VALID_LUA_TARGETS = __ERDE_TMP_6__["VALID_LUA_TARGETS"]
end
local io, string
do
	local __ERDE_TMP_9__
	__ERDE_TMP_9__ = require("erde.stdlib")
	io = __ERDE_TMP_9__["io"]
	string = __ERDE_TMP_9__["string"]
end
local echo, get_source_summary
do
	local __ERDE_TMP_12__
	__ERDE_TMP_12__ = require("erde.utils")
	echo = __ERDE_TMP_12__["echo"]
	get_source_summary = __ERDE_TMP_12__["get_source_summary"]
end
local loadlua = loadstring or load
local unpack = table.unpack or unpack
local native_traceback = debug.traceback
local searchers = package.loaders or package.searchers
local erde_source_cache = {}
local erde_source_id_counter = 1
local function rewrite(message)
	if type(message) ~= "string" then
		return message
	end
	for erde_source_id, chunkname, compiled_line in message:gmatch('%[string "erde::(%d+)::([^\n]+)"]:(%d+)') do
		local cache = erde_source_cache[tonumber(erde_source_id)] or {}
		local source_map = cache.source_map or {}
		local source_line = source_map[tonumber(compiled_line)] or ("(compiled:" .. tostring(compiled_line) .. ")")
		local match = string.escape(
			(
					'[string "erde::'
					.. tostring(erde_source_id)
					.. "::"
					.. tostring(chunkname)
					.. '"]:'
					.. tostring(compiled_line)
				)
		)
		message = cache.has_alias and message:gsub(match, chunkname .. ":" .. source_line)
			or message:gsub(match, ('[string "' .. tostring(chunkname) .. '"]:' .. tostring(source_line)))
	end
	message = message:gsub("__ERDE_SUBSTITUTE_([a-zA-Z]+)__", "%1")
	return message
end
_MODULE.rewrite = rewrite
local function traceback(arg1, arg2, arg3)
	local stacktrace, level
	if type(arg1) == "thread" then
		level = arg3 or 1
		stacktrace = native_traceback(arg1, arg2, level + 1)
	else
		level = arg2 or 1
		stacktrace = native_traceback(arg1, level + 1)
	end
	if type(stacktrace) ~= "string" then
		return stacktrace
	end
	if level > -1 and config.is_cli_runtime then
		local stack = string.split(stacktrace, "\n")
		local stacklen = #stack
		for i = 1, 4 do
			table.remove(stack, stacklen - i)
		end
		stacktrace = table.concat(stack, "\n")
	end
	stacktrace = stacktrace:gsub(
		table.concat({
			"[^\n]*\n",
			"[^\n]*__erde_internal_load_source__[^\n]*\n",
			"[^\n]*\n",
		}),
		""
	)
	return rewrite(stacktrace)
end
_MODULE.traceback = traceback
local function __erde_internal_load_source__(source, options)
	if options == nil then
		options = {}
	end
	local chunkname = table.concat({
		"erde",
		erde_source_id_counter,
		options.alias or get_source_summary(source),
	}, "::")
	local compiled, source_map = compile(source, {
		alias = options.alias,
		lua_target = options.lua_target,
		bitlib = options.bitlib,
	})
	compiled = compiled:gsub("^#![^\n]+", "")
	local loader, load_error = loadlua(compiled, chunkname)
	if load_error ~= nil then
		error(table.concat({
			"Failed to load compiled code:",
			tostring(load_error),
			"",
			"This is an internal error that should never happen.",
			"Please report this at: https://github.com/erde-lang/erde/issues",
			"",
			"erde",
			"----",
			source,
			"",
			"lua",
			"---",
			compiled,
		}, "\n"))
	end
	erde_source_cache[erde_source_id_counter] = {
		has_alias = options.alias ~= nil,
	}
	if not config.disable_source_maps and not options.disable_source_maps then
		erde_source_cache[erde_source_id_counter].source_map = source_map
	end
	erde_source_id_counter = erde_source_id_counter + 1
	return loader()
end
_MODULE.__erde_internal_load_source__ = __erde_internal_load_source__
local function run(source, options)
	return echo(__erde_internal_load_source__(source, options))
end
_MODULE.run = run
local function erde_searcher(module_name)
	local module_path = module_name:gsub("%.", PATH_SEPARATOR)
	for path in package.path:gmatch("[^;]+") do
		local fullpath = path:gsub("%.lua$", ".erde"):gsub("?", module_path)
		if io.exists(fullpath) then
			return function()
				local source = io.readfile(fullpath)
				local result = {
					__erde_internal_load_source__(source, {
						alias = fullpath,
					}),
				}
				return unpack(result)
			end
		end
	end
end
local function load(arg1, arg2)
	local new_lua_target, options = nil, {}
	if type(arg1) == "string" then
		new_lua_target = arg1
	end
	if type(arg1) == "table" then
		options = arg1
	elseif type(arg2) == "table" then
		options = arg2
	end
	config.bitlib = options.bitlib
	config.disable_source_maps = options.disable_source_maps
	debug.traceback = options.keep_traceback == true and native_traceback or traceback
	if new_lua_target ~= nil then
		if VALID_LUA_TARGETS[new_lua_target] then
			config.lua_target = new_lua_target
		else
			error(table.concat({
				("Invalid Lua target: " .. tostring(new_lua_target)),
				("Must be one of: " .. tostring(table.concat(VALID_LUA_TARGETS, ", "))),
			}, "\n"))
		end
	elseif jit ~= nil then
		config.lua_target = "jit"
	else
		new_lua_target = _VERSION:match("Lua (%d%.%d)")
		if VALID_LUA_TARGETS[new_lua_target] then
			config.lua_target = new_lua_target
		else
			error(("Unsupported Lua version: " .. tostring(_VERSION)))
		end
	end
	for _, searcher in ipairs(searchers) do
		if searcher == erde_searcher then
			return
		end
	end
	table.insert(searchers, 2, erde_searcher)
end
_MODULE.load = load
local function unload()
	debug.traceback = native_traceback
	for i, searcher in ipairs(searchers) do
		if searcher == erde_searcher then
			table.remove(searchers, i)
			return
		end
	end
end
_MODULE.unload = unload
return _MODULE
-- Compiled with Erde 0.6.0-1
-- __ERDE_COMPILED__

end,

["erde.config"] = function()
--------------------
-- Module: 'erde.config'
--------------------
local _MODULE = {}
local lua_target = "5.1+"
_MODULE.lua_target = lua_target
local is_cli_runtime = false
_MODULE.is_cli_runtime = is_cli_runtime
local bitlib = nil
_MODULE.bitlib = bitlib
local disable_source_maps = false
_MODULE.disable_source_maps = disable_source_maps
return _MODULE
-- Compiled with Erde 0.6.0-1
-- __ERDE_COMPILED__

end,

["erde.compile"] = function()
--------------------
-- Module: 'erde.compile'
--------------------
local config = require("erde.config")
local BINOP_ASSIGNMENT_TOKENS, BINOPS, BITOPS, BITLIB_METHODS, COMPILED_FOOTER_COMMENT, DIGIT, KEYWORDS, LEFT_ASSOCIATIVE, LUA_KEYWORDS, SURROUND_ENDS, TERMINALS, TOKEN_TYPES, UNOPS, VERSION
do
	local __ERDE_TMP_4__
	__ERDE_TMP_4__ = require("erde.constants")
	BINOP_ASSIGNMENT_TOKENS = __ERDE_TMP_4__["BINOP_ASSIGNMENT_TOKENS"]
	BINOPS = __ERDE_TMP_4__["BINOPS"]
	BITOPS = __ERDE_TMP_4__["BITOPS"]
	BITLIB_METHODS = __ERDE_TMP_4__["BITLIB_METHODS"]
	COMPILED_FOOTER_COMMENT = __ERDE_TMP_4__["COMPILED_FOOTER_COMMENT"]
	DIGIT = __ERDE_TMP_4__["DIGIT"]
	KEYWORDS = __ERDE_TMP_4__["KEYWORDS"]
	LEFT_ASSOCIATIVE = __ERDE_TMP_4__["LEFT_ASSOCIATIVE"]
	LUA_KEYWORDS = __ERDE_TMP_4__["LUA_KEYWORDS"]
	SURROUND_ENDS = __ERDE_TMP_4__["SURROUND_ENDS"]
	TERMINALS = __ERDE_TMP_4__["TERMINALS"]
	TOKEN_TYPES = __ERDE_TMP_4__["TOKEN_TYPES"]
	UNOPS = __ERDE_TMP_4__["UNOPS"]
	VERSION = __ERDE_TMP_4__["VERSION"]
end
local table
do
	local __ERDE_TMP_7__
	__ERDE_TMP_7__ = require("erde.stdlib")
	table = __ERDE_TMP_7__["table"]
end
local tokenize = require("erde.tokenize")
local get_source_alias
do
	local __ERDE_TMP_12__
	__ERDE_TMP_12__ = require("erde.utils")
	get_source_alias = __ERDE_TMP_12__["get_source_alias"]
end
local unpack = table.unpack or unpack
local arrow_function, block, expression, statement
local tokens
local current_token_index
local current_token
local block_depth
local tmp_name_counter
local break_name
local has_continue
local has_module_declarations
local is_module_return_block, module_return_line
local is_varargs_block
local block_declarations, block_declaration_stack
local goto_jumps, goto_labels
local alias
local lua_target
local bitlib
local function throw(message, line)
	if line == nil then
		line = current_token.line
	end
	error((tostring(alias) .. ":" .. tostring(line) .. ": " .. tostring(message)), 0)
end
local function add_block_declaration(var, scope, stack_depth)
	if stack_depth == nil then
		stack_depth = block_depth
	end
	if block_declaration_stack[stack_depth] == nil then
		for i = stack_depth - 1, 1, -1 do
			local parent_block_declarations = block_declaration_stack[i]
			if parent_block_declarations ~= nil then
				block_declaration_stack[stack_depth] = table.shallowcopy(parent_block_declarations)
				break
			end
		end
	end
	local target_block_declarations = block_declaration_stack[stack_depth]
	if type(var) == "string" then
		target_block_declarations[var] = scope
	else
		for _, declaration_name in ipairs(var.declaration_names) do
			target_block_declarations[declaration_name] = scope
		end
	end
end
local function consume()
	local consumed_token_value = current_token.value
	current_token_index = current_token_index + 1
	current_token = tokens[current_token_index]
	return consumed_token_value
end
local function branch(token)
	if token == current_token.value then
		consume()
		return true
	end
end
local function expect(token, should_consume)
	if current_token.type == TOKEN_TYPES.EOF then
		throw(("unexpected eof (expected " .. tostring(token) .. ")"))
	end
	if token ~= current_token.value then
		throw(("expected '" .. tostring(token) .. "' got '" .. tostring(current_token.value) .. "'"))
	end
	if should_consume then
		return consume()
	end
end
local function look_past_surround(token_start_index)
	if token_start_index == nil then
		token_start_index = current_token_index
	end
	local surround_start_token = tokens[token_start_index]
	local surround_end = SURROUND_ENDS[surround_start_token.value]
	local surround_depth = 1
	local look_ahead_token_index = token_start_index + 1
	local look_ahead_token = tokens[look_ahead_token_index]
	repeat
		if look_ahead_token.type == TOKEN_TYPES.EOF then
			throw(("unexpected eof, missing '" .. tostring(surround_end) .. "'"), surround_start_token.line)
		end
		if look_ahead_token.value == surround_start_token.value then
			surround_depth = surround_depth + 1
		elseif look_ahead_token.value == surround_end then
			surround_depth = surround_depth - 1
		end
		look_ahead_token_index = look_ahead_token_index + 1
		look_ahead_token = tokens[look_ahead_token_index]
	until surround_depth == 0
	return look_ahead_token, look_ahead_token_index
end
local function new_tmp_name()
	tmp_name_counter = tmp_name_counter + 1
	return ("__ERDE_TMP_" .. tostring(tmp_name_counter) .. "__")
end
local function get_compile_name(name, scope)
	if scope == "module" then
		if LUA_KEYWORDS[name] then
			return ("_MODULE['" .. tostring(name) .. "']")
		else
			return "_MODULE." .. name
		end
	elseif scope == "global" then
		if LUA_KEYWORDS[name] then
			return ("_G['" .. tostring(name) .. "']")
		else
			return "_G." .. name
		end
	end
	if LUA_KEYWORDS[name] then
		return (tostring(name) .. "_")
	else
		return name
	end
end
local function weave(t, separator)
	if separator == nil then
		separator = ","
	end
	local woven = {}
	local len = #t
	for i = 1, len - 1 do
		table.insert(woven, t[i])
		if type(t[i]) ~= "number" then
			table.insert(woven, separator)
		end
	end
	table.insert(woven, t[len])
	return woven
end
local function compile_binop(op_token, op_line, lhs, rhs)
	local needs_floor_division_polyfill = (
		op_token == "//"
		and lua_target ~= "5.3"
		and lua_target ~= "5.4"
		and lua_target ~= "5.3+"
		and lua_target ~= "5.4+"
	)
	if needs_floor_division_polyfill then
		return {
			op_line,
			"math.floor(",
			lhs,
			op_line,
			"/",
			rhs,
			")",
		}
	elseif bitlib and BITOPS[op_token] then
		return {
			op_line,
			("(require('" .. tostring(bitlib) .. "')." .. tostring(BITLIB_METHODS[op_token]) .. "("),
			lhs,
			",",
			rhs,
			"))",
		}
	elseif op_token == "!=" then
		return {
			lhs,
			"~=",
			rhs,
		}
	elseif op_token == "==" then
		return {
			lhs,
			op_token,
			rhs,
		}
	elseif op_token == "||" then
		return {
			lhs,
			"or",
			rhs,
		}
	elseif op_token == "&&" then
		return {
			lhs,
			"and",
			rhs,
		}
	else
		return {
			lhs,
			op_line,
			op_token,
			rhs,
		}
	end
end
local function list(callback, break_token)
	local list = {}
	repeat
		table.insert(list, callback() or nil)
	until not branch(",") or (break_token and current_token.value == break_token)
	return list
end
local function surround(open_char, close_char, callback)
	expect(open_char, true)
	local result = callback()
	expect(close_char, true)
	return result
end
local function surround_list(open_char, close_char, allow_empty, callback)
	return surround(open_char, close_char, function()
		if current_token.value ~= close_char or not allow_empty then
			return list(callback, close_char)
		else
			return {}
		end
	end)
end
local function name()
	if current_token.type == TOKEN_TYPES.EOF then
		throw("unexpected eof")
	end
	if current_token.type ~= TOKEN_TYPES.WORD then
		throw(("unexpected token '" .. tostring(current_token.value) .. "'"))
	end
	if KEYWORDS[current_token.value] ~= nil then
		throw(("unexpected keyword '" .. tostring(current_token.value) .. "'"))
	end
	if TERMINALS[current_token.value] ~= nil then
		throw(("unexpected builtin '" .. tostring(current_token.value) .. "'"))
	end
	return consume()
end
local function array_destructure(scope)
	local compile_lines = {}
	local compile_name = new_tmp_name()
	local declaration_names = {}
	local array_index = 0
	table.insert(compile_lines, current_token.line)
	surround_list("[", "]", false, function()
		array_index = array_index + 1
		local declaration_name = name()
		table.insert(declaration_names, declaration_name)
		local assignment_name = get_compile_name(declaration_name, scope)
		table.insert(
			compile_lines,
			(tostring(assignment_name) .. " = " .. tostring(compile_name) .. "[" .. tostring(array_index) .. "]")
		)
		if branch("=") then
			table.insert(
				compile_lines,
				("if " .. tostring(assignment_name) .. " == nil then " .. tostring(assignment_name) .. " = ")
			)
			table.insert(compile_lines, expression())
			table.insert(compile_lines, "end")
		end
	end)
	return {
		compile_lines = compile_lines,
		compile_name = compile_name,
		declaration_names = declaration_names,
	}
end
local function map_destructure(scope)
	local compile_lines = {}
	local compile_name = new_tmp_name()
	local declaration_names = {}
	table.insert(compile_lines, current_token.line)
	surround_list("{", "}", false, function()
		local key = name()
		local declaration_name = branch(":") and name() or key
		table.insert(declaration_names, declaration_name)
		local assignment_name = get_compile_name(declaration_name, scope)
		if LUA_KEYWORDS[declaration_name] then
			table.insert(
				compile_lines,
				(tostring(assignment_name) .. " = " .. tostring(compile_name) .. "['" .. tostring(key) .. "']")
			)
		else
			table.insert(
				compile_lines,
				(tostring(assignment_name) .. " = " .. tostring(compile_name) .. "." .. tostring(key))
			)
		end
		if branch("=") then
			table.insert(
				compile_lines,
				("if " .. tostring(assignment_name) .. " == nil then " .. tostring(assignment_name) .. " = ")
			)
			table.insert(compile_lines, expression())
			table.insert(compile_lines, "end")
		end
	end)
	return {
		compile_lines = compile_lines,
		compile_name = compile_name,
		declaration_names = declaration_names,
	}
end
local function variable(scope)
	if scope == nil then
		scope = "local"
	end
	if current_token.value == "[" then
		return array_destructure(scope)
	elseif current_token.value == "{" then
		return map_destructure(scope)
	else
		return name()
	end
end
local function bracket_index(index_chain_state)
	local compile_lines = {
		current_token.line,
		"[",
		surround("[", "]", expression),
		"]",
	}
	index_chain_state.final_base_compile_lines = table.shallowcopy(index_chain_state.compile_lines)
	index_chain_state.final_index_compile_lines = compile_lines
	table.insert(index_chain_state.compile_lines, compile_lines)
end
local function dot_index(index_chain_state)
	local compile_lines = {
		current_token.line,
	}
	consume()
	local key = name()
	if LUA_KEYWORDS[key] then
		table.insert(compile_lines, ("['" .. tostring(key) .. "']"))
	else
		table.insert(compile_lines, "." .. key)
	end
	index_chain_state.final_base_compile_lines = table.shallowcopy(index_chain_state.compile_lines)
	index_chain_state.final_index_compile_lines = compile_lines
	table.insert(index_chain_state.compile_lines, compile_lines)
end
local function method_index(index_chain_state)
	table.insert(index_chain_state.compile_lines, current_token.line)
	consume()
	local method_name = name()
	local method_parameters = surround_list("(", ")", true, expression)
	if not LUA_KEYWORDS[method_name] then
		table.insert(index_chain_state.compile_lines, (":" .. tostring(method_name) .. "("))
		table.insert(index_chain_state.compile_lines, weave(method_parameters))
		table.insert(index_chain_state.compile_lines, ")")
	elseif index_chain_state.has_trivial_base and index_chain_state.chain_len == 0 then
		table.insert(index_chain_state.compile_lines, ("['" .. tostring(method_name) .. "']("))
		table.insert(method_parameters, 1, index_chain_state.base_compile_lines)
		table.insert(index_chain_state.compile_lines, weave(method_parameters))
		table.insert(index_chain_state.compile_lines, ")")
	else
		index_chain_state.needs_block_compile = true
		table.insert(index_chain_state.block_compile_lines, index_chain_state.block_compile_name .. "=")
		table.insert(index_chain_state.block_compile_lines, index_chain_state.compile_lines)
		table.insert(method_parameters, 1, index_chain_state.block_compile_name)
		index_chain_state.compile_lines = {
			(tostring(index_chain_state.block_compile_name) .. "['" .. tostring(method_name) .. "']("),
			weave(method_parameters),
			")",
		}
	end
end
local function function_call_index(index_chain_state)
	local preceding_compile_lines = index_chain_state.compile_lines
	local preceding_compile_lines_len = #preceding_compile_lines
	while type(preceding_compile_lines[preceding_compile_lines_len]) == "table" do
		preceding_compile_lines = preceding_compile_lines[preceding_compile_lines_len]
		preceding_compile_lines_len = #preceding_compile_lines
	end
	preceding_compile_lines[preceding_compile_lines_len] = preceding_compile_lines[preceding_compile_lines_len] .. "("
	table.insert(index_chain_state.compile_lines, weave(surround_list("(", ")", true, expression)))
	table.insert(index_chain_state.compile_lines, ")")
end
local function index_chain(options)
	local block_compile_name = new_tmp_name()
	local index_chain_state = {
		base_compile_lines = options.base_compile_lines,
		compile_lines = {
			options.base_compile_lines,
		},
		has_trivial_base = options.has_trivial_base,
		chain_len = 0,
		is_function_call = false,
		needs_block_compile = false,
		block_compile_name = block_compile_name,
		block_compile_lines = {
			"local " .. block_compile_name,
		},
		final_base_compile_lines = options.base_compile_lines,
		final_index_compile_lines = {},
	}
	if options.wrap_base_compile_lines then
		table.insert(index_chain_state.compile_lines, 1, "(")
		table.insert(index_chain_state.compile_lines, ")")
	end
	while true do
		if current_token.value == "(" and current_token.line == tokens[current_token_index - 1].line then
			index_chain_state.is_function_call = true
			function_call_index(index_chain_state)
		elseif current_token.value == "[" then
			index_chain_state.is_function_call = false
			bracket_index(index_chain_state)
		elseif current_token.value == "." then
			index_chain_state.is_function_call = false
			dot_index(index_chain_state)
		elseif current_token.value == ":" then
			index_chain_state.is_function_call = true
			method_index(index_chain_state)
		else
			break
		end
		index_chain_state.chain_len = index_chain_state.chain_len + 1
	end
	if options.require_chain and index_chain_state.chain_len == 0 then
		if current_token.type == TOKEN_TYPES.EOF then
			throw("unexpected eof")
		else
			throw(("unexpected token '" .. tostring(current_token.value) .. "'"))
		end
	end
	if index_chain_state.chain_len == 0 then
		index_chain_state.compile_lines = options.base_compile_lines
	end
	return index_chain_state
end
local function single_quote_string()
	consume()
	if current_token.type == TOKEN_TYPES.SINGLE_QUOTE_STRING then
		return {
			current_token.line,
			"'" .. consume(),
		}
	else
		return {
			current_token.line,
			"'" .. consume() .. consume(),
		}
	end
end
local function double_quote_string()
	local double_quote_string_line = current_token.line
	local has_interpolation = false
	consume()
	if current_token.type == TOKEN_TYPES.DOUBLE_QUOTE_STRING then
		return {
			double_quote_string_line,
			'"' .. consume(),
		}, has_interpolation
	end
	local compile_lines = {}
	local content = ""
	repeat
		if current_token.type == TOKEN_TYPES.INTERPOLATION then
			has_interpolation = true
			if content ~= "" then
				table.insert(compile_lines, '"' .. content .. '"')
			end
			table.insert(compile_lines, {
				"tostring(",
				surround("{", "}", expression),
				")",
			})
			content = ""
		else
			content = content .. consume()
		end
	until current_token.type == TOKEN_TYPES.DOUBLE_QUOTE_STRING
	if content ~= "" then
		table.insert(compile_lines, '"' .. content .. '"')
	end
	consume()
	return {
		double_quote_string_line,
		weave(compile_lines, ".."),
	}, has_interpolation
end
local function block_string()
	local block_string_line = current_token.line
	local has_interpolation = false
	local start_quote = "[" .. current_token.equals .. "["
	local end_quote = "]" .. current_token.equals .. "]"
	consume()
	if current_token.type == TOKEN_TYPES.BLOCK_STRING then
		consume()
		return {
			block_string_line,
			start_quote .. end_quote,
		}, has_interpolation
	end
	local compile_lines = {}
	local content = ""
	repeat
		if current_token.type == TOKEN_TYPES.INTERPOLATION then
			has_interpolation = true
			if content ~= "" then
				table.insert(compile_lines, start_quote .. content .. end_quote)
			end
			table.insert(compile_lines, {
				"tostring(",
				surround("{", "}", expression),
				")",
			})
			content = ""
			if current_token.value:sub(1, 1) == "\n" then
				content = content .. "\n" .. consume()
			end
		else
			content = content .. consume()
		end
	until current_token.type == TOKEN_TYPES.BLOCK_STRING
	if content ~= "" then
		table.insert(compile_lines, start_quote .. content .. end_quote)
	end
	consume()
	return {
		block_string_line,
		weave(compile_lines, ".."),
	}, has_interpolation
end
local function table_constructor()
	local table_constructor_line = current_token.line
	local compile_lines = {}
	surround_list("{", "}", true, function()
		local next_token = tokens[current_token_index + 1]
		if current_token.value == "[" then
			table.insert(compile_lines, "[")
			table.insert(compile_lines, surround("[", "]", expression))
			table.insert(compile_lines, "]")
			table.insert(compile_lines, expect("=", true))
		elseif next_token.type == TOKEN_TYPES.SYMBOL and next_token.value == "=" then
			local key = name()
			if LUA_KEYWORDS[key] then
				table.insert(compile_lines, ("['" .. tostring(key) .. "']") .. consume())
			else
				table.insert(compile_lines, key .. consume())
			end
		end
		table.insert(compile_lines, expression())
		table.insert(compile_lines, ",")
	end)
	return {
		table_constructor_line,
		"{",
		compile_lines,
		"}",
	}
end
local function return_list()
	local look_ahead_limit_token, look_ahead_limit_token_index = look_past_surround()
	if look_ahead_limit_token.value == "->" or look_ahead_limit_token.value == "=>" then
		return arrow_function()
	end
	local look_ahead_token_index = current_token_index + 1
	local look_ahead_token = tokens[look_ahead_token_index]
	while look_ahead_token_index < look_ahead_limit_token_index do
		if look_ahead_token.type == TOKEN_TYPES.SYMBOL and SURROUND_ENDS[look_ahead_token.value] then
			look_ahead_token, look_ahead_token_index = look_past_surround(look_ahead_token_index)
		elseif look_ahead_token.type == TOKEN_TYPES.SYMBOL and look_ahead_token.value == "," then
			return weave(surround_list("(", ")", false, expression))
		else
			look_ahead_token_index = look_ahead_token_index + 1
			look_ahead_token = tokens[look_ahead_token_index]
		end
	end
	return expression()
end
local function block_return()
	if is_module_return_block then
		module_return_line = current_token.line
	end
	local compile_lines = {
		consume(),
	}
	if block_depth == 1 then
		if current_token.type ~= TOKEN_TYPES.EOF then
			if current_token.value == "(" then
				table.insert(compile_lines, return_list())
			else
				table.insert(compile_lines, weave(list(expression)))
			end
		end
		if current_token.type ~= TOKEN_TYPES.EOF then
			throw(("expected '<eof>', got '" .. tostring(current_token.value) .. "'"))
		end
	else
		if current_token.value ~= "}" then
			if current_token.value == "(" then
				table.insert(compile_lines, return_list())
			else
				table.insert(compile_lines, weave(list(expression)))
			end
		end
		if current_token.value ~= "}" then
			throw(("expected '}', got '" .. tostring(current_token.value) .. "'"))
		end
	end
	return compile_lines
end
local function parameters()
	local compile_lines = {}
	local compile_names = {}
	local has_varargs = false
	surround_list("(", ")", true, function()
		if branch("...") then
			has_varargs = true
			table.insert(compile_names, "...")
			if current_token.type == TOKEN_TYPES.WORD then
				local varargs_name = name()
				table.insert(compile_lines, ("local " .. tostring(get_compile_name(varargs_name)) .. " = { ... }"))
				add_block_declaration(varargs_name, "local", block_depth + 1)
			end
			branch(",")
			expect(")")
		else
			local var = variable()
			add_block_declaration(var, "local", block_depth + 1)
			local compile_name = type(var) == "string" and get_compile_name(var) or var.compile_name
			table.insert(compile_names, compile_name)
			if branch("=") then
				table.insert(
					compile_lines,
					("if " .. tostring(compile_name) .. " == nil then " .. tostring(compile_name) .. " = ")
				)
				table.insert(compile_lines, expression())
				table.insert(compile_lines, "end")
			end
			if type(var) == "table" then
				table.insert(
					compile_lines,
					"local " .. table.concat(table.map(var.declaration_names, get_compile_name), ",")
				)
				table.insert(compile_lines, var.compile_lines)
			end
		end
	end)
	return {
		compile_lines = compile_lines,
		compile_names = compile_names,
		has_varargs = has_varargs,
	}
end
local function function_block()
	local old_is_module_return_block = is_module_return_block
	local old_break_name = break_name
	is_module_return_block = false
	break_name = nil
	local compile_lines = block()
	is_module_return_block = old_is_module_return_block
	break_name = old_break_name
	return compile_lines
end
function arrow_function()
	local old_is_varargs_block = is_varargs_block
	local arrow_function_line = current_token.line
	local param_compile_lines = {}
	local param_compile_names = {}
	if current_token.value == "(" then
		local params = parameters()
		table.insert(param_compile_lines, params.compile_lines)
		is_varargs_block = params.has_varargs
		param_compile_names = params.compile_names
	else
		is_varargs_block = false
		local var = variable()
		add_block_declaration(var, "local", block_depth + 1)
		if type(var) == "string" then
			table.insert(param_compile_names, get_compile_name(var))
		else
			table.insert(param_compile_names, var.compile_name)
			table.insert(
				param_compile_lines,
				"local " .. table.concat(table.map(var.declaration_names, get_compile_name), ",")
			)
			table.insert(param_compile_lines, var.compile_lines)
		end
	end
	if current_token.value == "->" then
		consume()
	elseif current_token.value == "=>" then
		table.insert(param_compile_names, 1, "self")
		consume()
	elseif current_token.type == TOKEN_TYPES.EOF then
		throw("unexpected eof (expected '->' or '=>')")
	else
		throw(("unexpected token '" .. tostring(current_token.value) .. "' (expected '->' or '=>')"))
	end
	local compile_lines = {
		arrow_function_line,
		("function(" .. tostring(table.concat(param_compile_names, ",")) .. ")"),
		param_compile_lines,
	}
	if current_token.value == "{" then
		table.insert(compile_lines, surround("{", "}", function_block))
	else
		table.insert(compile_lines, "return")
		local old_block_declarations = block_declarations
		block_depth = block_depth + 1
		block_declaration_stack[block_depth] = block_declaration_stack[block_depth] or {}
		block_declarations = block_declaration_stack[block_depth]
		if current_token.value == "(" then
			table.insert(compile_lines, return_list())
		else
			table.insert(compile_lines, expression())
		end
		block_declarations = old_block_declarations
		block_declaration_stack[block_depth] = nil
		block_depth = block_depth - 1
	end
	table.insert(compile_lines, "end")
	is_varargs_block = old_is_varargs_block
	return compile_lines
end
local function function_signature(scope)
	local base_name = name()
	if (current_token.value == "." or current_token.value == ":") and scope ~= nil then
		throw("cannot use scopes for table values", current_token.line)
	end
	if scope == "module" or scope == "global" then
		block_declarations[base_name] = scope
	end
	local signature = get_compile_name(base_name, scope or block_declarations[base_name])
	local needs_label_assignment = false
	local needs_self_injection = false
	while branch(".") do
		local key = name()
		if LUA_KEYWORDS[key] then
			needs_label_assignment = true
			signature = signature .. ("['" .. tostring(key) .. "']")
		else
			signature = signature .. "." .. key
		end
	end
	if branch(":") then
		local key = name()
		if LUA_KEYWORDS[key] then
			needs_label_assignment = true
			needs_self_injection = true
			signature = signature .. ("['" .. tostring(key) .. "']")
		else
			signature = signature .. ":" .. key
		end
	end
	return {
		signature = signature,
		needs_label_assignment = needs_label_assignment,
		needs_self_injection = needs_self_injection,
	}
end
local function function_declaration(scope)
	local compile_lines = {
		current_token.line,
	}
	consume()
	local signature, needs_label_assignment, needs_self_injection
	do
		local __ERDE_TMP_979__
		__ERDE_TMP_979__ = function_signature(scope)
		signature = __ERDE_TMP_979__["signature"]
		needs_label_assignment = __ERDE_TMP_979__["needs_label_assignment"]
		needs_self_injection = __ERDE_TMP_979__["needs_self_injection"]
	end
	if scope == "local" then
		table.insert(compile_lines, "local")
	end
	if needs_label_assignment then
		table.insert(compile_lines, signature)
		table.insert(compile_lines, "=")
		table.insert(compile_lines, "function")
	else
		table.insert(compile_lines, "function")
		table.insert(compile_lines, signature)
	end
	local params = parameters()
	if needs_self_injection then
		table.insert(params.compile_names, "self")
	end
	table.insert(compile_lines, "(" .. table.concat(params.compile_names, ",") .. ")")
	table.insert(compile_lines, params.compile_lines)
	local old_is_varargs_block = is_varargs_block
	is_varargs_block = params.has_varargs
	table.insert(compile_lines, surround("{", "}", function_block))
	is_varargs_block = old_is_varargs_block
	table.insert(compile_lines, "end")
	return compile_lines
end
local function index_chain_expression(options)
	local index_chain = index_chain(options)
	if index_chain.needs_block_compile then
		return {
			"(function()",
			index_chain.block_compile_lines,
			"return",
			index_chain.compile_lines,
			"end)()",
		}
	else
		return index_chain.compile_lines
	end
end
local function terminal_expression()
	if current_token.type == TOKEN_TYPES.NUMBER then
		return {
			current_token.line,
			consume(),
		}
	elseif current_token.type == TOKEN_TYPES.SINGLE_QUOTE_STRING then
		return index_chain_expression({
			base_compile_lines = single_quote_string(),
			has_trivial_base = true,
			wrap_base_compile_lines = true,
		})
	elseif current_token.type == TOKEN_TYPES.DOUBLE_QUOTE_STRING then
		local compile_lines, has_interpolation = double_quote_string()
		return index_chain_expression({
			base_compile_lines = compile_lines,
			has_trivial_base = not has_interpolation,
			wrap_base_compile_lines = true,
		})
	elseif current_token.type == TOKEN_TYPES.BLOCK_STRING then
		local compile_lines, has_interpolation = block_string()
		return index_chain_expression({
			base_compile_lines = compile_lines,
			has_trivial_base = not has_interpolation,
			wrap_base_compile_lines = true,
		})
	end
	if TERMINALS[current_token.value] then
		if current_token.value == "..." and not is_varargs_block then
			throw("cannot use '...' outside a vararg function")
		end
		return {
			current_token.line,
			consume(),
		}
	end
	local next_token = tokens[current_token_index + 1]
	local is_arrow_function = (
		next_token.type == TOKEN_TYPES.SYMBOL and (next_token.value == "->" or next_token.value == "=>")
	)
	if not is_arrow_function and SURROUND_ENDS[current_token.value] then
		local past_surround_token = look_past_surround()
		is_arrow_function = (
			past_surround_token.type == TOKEN_TYPES.SYMBOL
			and (past_surround_token.value == "->" or past_surround_token.value == "=>")
		)
	end
	if is_arrow_function then
		return arrow_function()
	elseif current_token.value == "{" then
		return table_constructor()
	elseif current_token.value == "(" then
		return index_chain_expression({
			base_compile_lines = {
				current_token.line,
				"(",
				surround("(", ")", expression),
				")",
			},
		})
	else
		local base_name_line, base_name = current_token.line, name()
		return index_chain_expression({
			base_compile_lines = {
				base_name_line,
				get_compile_name(base_name, block_declarations[base_name]),
			},
			has_trivial_base = true,
		})
	end
end
local function unop_expression()
	local unop_line, unop = current_token.line, UNOPS[consume()]
	local operand = expression(unop.prec + 1)
	if unop.token == "!" then
		return {
			"not",
			operand,
		}
	elseif unop.token ~= "~" then
		return {
			unop_line,
			unop.token,
			operand,
		}
	elseif bitlib then
		return {
			unop_line,
			("(require('" .. tostring(bitlib) .. "').bnot("),
			operand,
			"))",
		}
	elseif lua_target == "5.1+" or lua_target == "5.2+" then
		throw("must specify bitlib for compiling bit operations when targeting 5.1+ or 5.2+", unop_line)
	else
		return {
			unop_line,
			unop.token,
			operand,
		}
	end
end
function expression(min_prec)
	if min_prec == nil then
		min_prec = 1
	end
	if current_token.type == TOKEN_TYPES.EOF then
		throw("unexpected eof (expected expression)")
	end
	local compile_lines = UNOPS[current_token.value] and unop_expression() or terminal_expression()
	local binop_line, binop = current_token.line, BINOPS[current_token.value]
	while binop and binop.prec >= min_prec do
		if BITOPS[binop.token] and (lua_target == "5.1+" or lua_target == "5.2+") and not bitlib then
			throw("must specify bitlib for compiling bit operations when targeting 5.1+ or 5.2+", binop_line)
		end
		consume()
		if binop.token == "~" and current_token.value == "=" then
			throw("unexpected token '~=', did you mean '!='?")
		end
		local operand = binop.assoc == LEFT_ASSOCIATIVE and expression(binop.prec + 1) or expression(binop.prec)
		compile_lines = compile_binop(binop.token, binop_line, compile_lines, operand)
		binop_line, binop = current_token.line, BINOPS[current_token.value]
	end
	return compile_lines
end
function block()
	local old_block_declarations = block_declarations
	block_depth = block_depth + 1
	block_declaration_stack[block_depth] = block_declaration_stack[block_depth]
		or table.shallowcopy(block_declaration_stack[block_depth - 1])
	block_declarations = block_declaration_stack[block_depth]
	local compile_lines = {}
	while current_token.value ~= "}" do
		table.insert(compile_lines, statement())
	end
	block_declarations = old_block_declarations
	block_declaration_stack[block_depth] = nil
	block_depth = block_depth - 1
	return compile_lines
end
local function do_block()
	return {
		consume(),
		surround("{", "}", block),
		"end",
	}
end
local function loop_block()
	local old_break_name = break_name
	local old_has_continue = has_continue
	break_name = new_tmp_name()
	has_continue = false
	local compile_lines = block()
	if has_continue then
		if lua_target == "5.1" or lua_target == "5.1+" then
			table.insert(compile_lines, 1, ("local " .. tostring(break_name) .. " = true repeat"))
			table.insert(
				compile_lines,
				(tostring(break_name) .. " = false until true if " .. tostring(break_name) .. " then break end")
			)
		else
			table.insert(compile_lines, ("::" .. tostring(break_name) .. "::"))
		end
	end
	break_name = old_break_name
	has_continue = old_has_continue
	return compile_lines
end
local function loop_break()
	consume()
	if break_name == nil then
		throw("cannot use 'break' outside of loop")
	end
	if lua_target == "5.1" or lua_target == "5.1+" or lua_target == "jit" then
		if current_token.value ~= "}" then
			throw(("expected '}', got '" .. tostring(current_token.value) .. "'"))
		end
	end
	return "break"
end
local function loop_continue()
	if break_name == nil then
		throw("cannot use 'continue' outside of loop")
	end
	has_continue = true
	consume()
	if lua_target == "5.1" or lua_target == "5.1+" then
		return (tostring(break_name) .. " = false do break end")
	else
		return ("goto " .. tostring(break_name))
	end
end
local function for_loop()
	local compile_lines = {}
	local pre_body_compile_lines = {}
	table.insert(compile_lines, consume())
	local next_token = tokens[current_token_index + 1]
	if next_token.type == TOKEN_TYPES.SYMBOL and next_token.value == "=" then
		local loop_name = name()
		add_block_declaration(loop_name, "local", block_depth + 1)
		table.insert(compile_lines, get_compile_name(loop_name) .. consume())
		local expressions_line = current_token.line
		local expressions = list(expression)
		local num_expressions = #expressions
		if num_expressions ~= 2 and num_expressions ~= 3 then
			throw(
				("invalid numeric for, expected 2-3 expressions, got " .. tostring(num_expressions)),
				expressions_line
			)
		end
		table.insert(compile_lines, weave(expressions))
	else
		local names = {}
		for _, var in ipairs(list(variable)) do
			add_block_declaration(var, "local", block_depth + 1)
			if type(var) == "string" then
				table.insert(names, get_compile_name(var))
			else
				table.insert(names, var.compile_name)
				table.insert(
					pre_body_compile_lines,
					"local " .. table.concat(table.map(var.declaration_names, get_compile_name), ",")
				)
				table.insert(pre_body_compile_lines, var.compile_lines)
			end
		end
		table.insert(compile_lines, weave(names))
		table.insert(compile_lines, expect("in", true))
		table.insert(compile_lines, weave(list(expression)))
	end
	table.insert(compile_lines, "do")
	table.insert(compile_lines, pre_body_compile_lines)
	table.insert(compile_lines, surround("{", "}", loop_block))
	table.insert(compile_lines, "end")
	return compile_lines
end
local function repeat_until()
	return {
		consume(),
		surround("{", "}", loop_block),
		expect("until", true),
		expression(),
	}
end
local function while_loop()
	return {
		consume(),
		expression(),
		"do",
		surround("{", "}", loop_block),
		"end",
	}
end
local function goto_jump()
	if lua_target == "5.1" or lua_target == "5.1+" then
		throw("'goto' statements only compatibly with lua targets 5.2+, jit")
	end
	consume()
	local label_line, label = current_token.line, name()
	table.insert(goto_jumps, {
		label = label,
		line = label_line,
	})
	return "goto " .. get_compile_name(label)
end
local function goto_label()
	if lua_target == "5.1" or lua_target == "5.1+" then
		throw("'goto' statements only compatibly with lua targets 5.2+, jit")
	end
	consume()
	local label = name()
	goto_labels[label] = true
	return "::" .. get_compile_name(label) .. expect("::", true)
end
local function if_else()
	local compile_lines = {}
	table.insert(compile_lines, consume())
	table.insert(compile_lines, expression())
	table.insert(compile_lines, "then")
	table.insert(compile_lines, surround("{", "}", block))
	while current_token.value == "elseif" do
		table.insert(compile_lines, consume())
		table.insert(compile_lines, expression())
		table.insert(compile_lines, "then")
		table.insert(compile_lines, surround("{", "}", block))
	end
	if current_token.value == "else" then
		table.insert(compile_lines, consume())
		if current_token.value == "if" then
			throw("unexpected tokens 'else if', did you mean 'elseif'?")
		end
		table.insert(compile_lines, surround("{", "}", block))
	end
	table.insert(compile_lines, "end")
	return compile_lines
end
local function assignment_index_chain()
	if current_token.value == "(" then
		return index_chain({
			base_compile_lines = {
				current_token.line,
				"(",
				surround("(", ")", expression),
				")",
			},
			require_chain = true,
		})
	else
		local base_name_line, base_name = current_token.line, name()
		return index_chain({
			base_compile_lines = {
				base_name_line,
				get_compile_name(base_name, block_declarations[base_name]),
			},
			has_trivial_base = true,
		})
	end
end
local function non_operator_assignment(ids, expressions)
	local assignment_ids = {}
	local assignment_block_compile_names = {}
	local assignment_block_compile_lines = {}
	for _, id in ipairs(ids) do
		if not id.needs_block_compile then
			table.insert(assignment_ids, id.compile_lines)
		else
			local assignment_name = new_tmp_name()
			table.insert(assignment_ids, assignment_name)
			table.insert(assignment_block_compile_names, assignment_name)
			table.insert(assignment_block_compile_lines, id.block_compile_lines)
			table.insert(assignment_block_compile_lines, id.compile_lines)
			table.insert(assignment_block_compile_lines, "=" .. assignment_name)
		end
	end
	local compile_lines = {}
	if #assignment_block_compile_names > 0 then
		table.insert(compile_lines, "local")
		table.insert(compile_lines, weave(assignment_block_compile_names))
	end
	table.insert(compile_lines, weave(assignment_ids))
	table.insert(compile_lines, "=")
	table.insert(compile_lines, weave(expressions))
	table.insert(compile_lines, assignment_block_compile_lines)
	return compile_lines
end
local function single_operator_assignment(id, expr, op_token, op_line)
	local compile_lines = {}
	local id_compile_lines = id.compile_lines
	if id.needs_block_compile then
		table.insert(compile_lines, id.block_compile_lines)
	end
	if not id.has_trivial_base or id.chain_len > 1 then
		local final_base_name = new_tmp_name()
		table.insert(compile_lines, ("local " .. tostring(final_base_name) .. " ="))
		table.insert(compile_lines, id.final_base_compile_lines)
		id_compile_lines = {
			final_base_name,
			id.final_index_compile_lines,
		}
	end
	table.insert(compile_lines, id_compile_lines)
	table.insert(compile_lines, "=")
	if type(expr) == "table" then
		expr = {
			"(",
			expr,
			")",
		}
	end
	table.insert(compile_lines, compile_binop(op_token, op_line, id_compile_lines, expr))
	return compile_lines
end
local function operator_assignment(ids, expressions, op_token, op_line)
	if #ids == 1 then
		return single_operator_assignment(ids[1], expressions[1], op_token, op_line)
	end
	local assignment_names = {}
	local assignment_compile_lines = {}
	for _, id in ipairs(ids) do
		local assignment_name = new_tmp_name()
		table.insert(assignment_names, assignment_name)
		table.insert(assignment_compile_lines, single_operator_assignment(id, assignment_name, op_token, op_line))
	end
	return {
		("local " .. tostring(table.concat(assignment_names, ",")) .. " ="),
		weave(expressions),
		assignment_compile_lines,
	}
end
local function variable_assignment(first_id)
	local ids = {
		first_id,
	}
	while branch(",") do
		local id_line, id = current_token.line, assignment_index_chain()
		if id.is_function_call then
			throw("cannot assign value to function call", id_line)
		end
		table.insert(ids, id)
	end
	local op_line, op_token = current_token.line, BINOP_ASSIGNMENT_TOKENS[current_token.value] and consume()
	if BITOPS[op_token] and (lua_target == "5.1+" or lua_target == "5.2+") and not bitlib then
		throw("must specify bitlib for compiling bit operations when targeting 5.1+ or 5.2+", op_line)
	end
	expect("=", true)
	local expressions = list(expression)
	if op_token then
		return operator_assignment(ids, expressions, op_token, op_line)
	else
		return non_operator_assignment(ids, expressions)
	end
end
local function variable_declaration(scope)
	local assignment_names = {}
	local has_destructure = false
	local destructure_declaration_names = {}
	local destructure_compile_lines = {}
	for _, var in
		ipairs(list(function()
			return variable(scope)
		end))
	do
		add_block_declaration(var, scope)
		if type(var) == "string" then
			table.insert(assignment_names, get_compile_name(var, scope))
		else
			has_destructure = true
			table.insert(assignment_names, var.compile_name)
			table.insert(destructure_compile_lines, var.compile_lines)
			if scope == "local" then
				table.merge(destructure_declaration_names, table.map(var.declaration_names, get_compile_name))
			else
				table.insert(destructure_declaration_names, var.compile_name)
			end
		end
	end
	if has_destructure then
		expect("=", true)
	elseif scope == "local" and current_token.value ~= "=" then
		return "local " .. table.concat(assignment_names, ",")
	elseif not branch("=") then
		return ""
	end
	local compile_lines = {}
	if has_destructure then
		table.insert(compile_lines, "local " .. table.concat(destructure_declaration_names, ","))
	end
	if scope == "local" then
		table.insert(compile_lines, "local")
	end
	table.insert(compile_lines, table.concat(assignment_names, ",") .. "=")
	table.insert(compile_lines, weave(list(expression)))
	table.insert(compile_lines, destructure_compile_lines)
	return compile_lines
end
function statement()
	local compile_lines = {}
	if current_token.value == "break" then
		table.insert(compile_lines, loop_break())
	elseif current_token.value == "continue" then
		table.insert(compile_lines, loop_continue())
	elseif current_token.value == "goto" then
		table.insert(compile_lines, goto_jump())
	elseif current_token.value == "::" then
		table.insert(compile_lines, goto_label())
	elseif current_token.value == "do" then
		table.insert(compile_lines, do_block())
	elseif current_token.value == "if" then
		table.insert(compile_lines, if_else())
	elseif current_token.value == "for" then
		table.insert(compile_lines, for_loop())
	elseif current_token.value == "while" then
		table.insert(compile_lines, while_loop())
	elseif current_token.value == "repeat" then
		table.insert(compile_lines, repeat_until())
	elseif current_token.value == "return" then
		table.insert(compile_lines, block_return())
	elseif current_token.value == "function" then
		table.insert(compile_lines, function_declaration())
	elseif current_token.value == "local" or current_token.value == "global" or current_token.value == "module" then
		local scope_line, scope = current_token.line, consume()
		if scope == "module" then
			if block_depth > 1 then
				throw("module declarations must appear at the top level", scope_line)
			end
			has_module_declarations = true
		end
		if current_token.value == "function" then
			table.insert(compile_lines, function_declaration(scope))
		else
			table.insert(compile_lines, variable_declaration(scope))
		end
	else
		local chain = assignment_index_chain()
		if not chain.is_function_call then
			table.insert(compile_lines, variable_assignment(chain))
		elseif chain.needs_block_compile then
			table.insert(compile_lines, "do")
			table.insert(compile_lines, chain.block_compile_lines)
			table.insert(compile_lines, chain.compile_lines)
			table.insert(compile_lines, "end")
		else
			table.insert(compile_lines, chain.compile_lines)
		end
	end
	if current_token.value == ";" then
		table.insert(compile_lines, consume())
	elseif current_token.value == "(" then
		table.insert(compile_lines, ";")
	end
	return compile_lines
end
local function module_block()
	local compile_lines = {}
	block_declarations = {}
	block_declaration_stack[block_depth] = block_declarations
	if current_token.type == TOKEN_TYPES.SHEBANG then
		table.insert(compile_lines, consume())
	end
	while current_token.type ~= TOKEN_TYPES.EOF do
		table.insert(compile_lines, statement())
	end
	if has_module_declarations then
		if module_return_line ~= nil then
			throw("cannot use 'return' w/ 'module' declarations", module_return_line)
		end
		table.insert(compile_lines, 1, "local _MODULE = {}")
		table.insert(compile_lines, "return _MODULE")
	end
	return compile_lines
end
local function collect_compile_lines(lines, state)
	for _, line in ipairs(lines) do
		if type(line) == "number" then
			state.source_line = line
		elseif type(line) == "string" then
			table.insert(state.compile_lines, line)
			table.insert(state.source_map, state.source_line)
		else
			collect_compile_lines(line, state)
		end
	end
end
return function(source, options)
	if options == nil then
		options = {}
	end
	tokens = tokenize(source, options.alias)
	current_token_index = 1
	current_token = tokens[current_token_index]
	block_depth = 1
	tmp_name_counter = 1
	break_name = nil
	has_continue = false
	has_module_declarations = false
	is_module_return_block = true
	module_return_line = nil
	is_varargs_block = true
	block_declarations = {}
	block_declaration_stack = {}
	goto_jumps = {}
	goto_labels = {}
	alias = options.alias or get_source_alias(source)
	lua_target = options.lua_target or config.lua_target
	bitlib = options.bitlib
		or config.bitlib
		or (lua_target == "5.1" and "bit")
		or (lua_target == "jit" and "bit")
		or (lua_target == "5.2" and "bit32")
	local source_map = {}
	local compile_lines = {}
	collect_compile_lines(module_block(), {
		compile_lines = compile_lines,
		source_map = source_map,
		source_line = current_token.line,
	})
	for _, __ERDE_TMP_1927__ in ipairs(goto_jumps) do
		local label, line
		label = __ERDE_TMP_1927__["label"]
		line = __ERDE_TMP_1927__["line"]
		if goto_labels[label] == nil then
			throw(("failed to find goto label '" .. tostring(label) .. "'"), line)
		end
	end
	table.insert(
		compile_lines,
		("-- Compiled with Erde " .. tostring(VERSION) .. " w/ Lua target " .. tostring(lua_target))
	)
	table.insert(compile_lines, COMPILED_FOOTER_COMMENT)
	return table.concat(compile_lines, "\n"), source_map
end
-- Compiled with Erde 0.6.0-1
-- __ERDE_COMPILED__

end,

["erde.constants"] = function()
--------------------
-- Module: 'erde.constants'
--------------------
local _MODULE = {}
local VERSION = "1.0.0-1"
_MODULE.VERSION = VERSION
local PATH_SEPARATOR = package.config:sub(1, 1)
_MODULE.PATH_SEPARATOR = PATH_SEPARATOR
local COMPILED_FOOTER_COMMENT = "-- __ERDE_COMPILED__"
_MODULE.COMPILED_FOOTER_COMMENT = COMPILED_FOOTER_COMMENT
local TOKEN_TYPES = {
	EOF = 0,
	SHEBANG = 1,
	SYMBOL = 2,
	WORD = 3,
	NUMBER = 4,
	SINGLE_QUOTE_STRING = 5,
	DOUBLE_QUOTE_STRING = 6,
	STRING_CONTENT = 7,
	INTERPOLATION = 8,
}
_MODULE.TOKEN_TYPES = TOKEN_TYPES
local VALID_LUA_TARGETS = {
	"jit",
	"5.1",
	"5.1+",
	"5.2",
	"5.2+",
	"5.3",
	"5.3+",
	"5.4",
	"5.4+",
}
_MODULE.VALID_LUA_TARGETS = VALID_LUA_TARGETS
for i, target in ipairs(VALID_LUA_TARGETS) do
	VALID_LUA_TARGETS[target] = true
end
local KEYWORDS = {
	["break"] = true,
	["continue"] = true,
	["do"] = true,
	["else"] = true,
	["elseif"] = true,
	["for"] = true,
	["function"] = true,
	["global"] = true,
	["if"] = true,
	["in"] = true,
	["local"] = true,
	["module"] = true,
	["repeat"] = true,
	["return"] = true,
	["until"] = true,
	["while"] = true,
}
_MODULE.KEYWORDS = KEYWORDS
local LUA_KEYWORDS = {
	["not"] = true,
	["and"] = true,
	["or"] = true,
	["end"] = true,
	["then"] = true,
}
_MODULE.LUA_KEYWORDS = LUA_KEYWORDS
local TERMINALS = {
	["true"] = true,
	["false"] = true,
	["nil"] = true,
	["..."] = true,
}
_MODULE.TERMINALS = TERMINALS
local LEFT_ASSOCIATIVE = -1
_MODULE.LEFT_ASSOCIATIVE = LEFT_ASSOCIATIVE
local RIGHT_ASSOCIATIVE = 1
_MODULE.RIGHT_ASSOCIATIVE = RIGHT_ASSOCIATIVE
local UNOPS = {
	["-"] = {
		prec = 13,
	},
	["#"] = {
		prec = 13,
	},
	["!"] = {
		prec = 13,
	},
	["~"] = {
		prec = 13,
	},
}
_MODULE.UNOPS = UNOPS
for token, op in pairs(UNOPS) do
	op.token = token
end
local BITOPS = {
	["|"] = {
		prec = 6,
		assoc = LEFT_ASSOCIATIVE,
	},
	["~"] = {
		prec = 7,
		assoc = LEFT_ASSOCIATIVE,
	},
	["&"] = {
		prec = 8,
		assoc = LEFT_ASSOCIATIVE,
	},
	["<<"] = {
		prec = 9,
		assoc = LEFT_ASSOCIATIVE,
	},
	[">>"] = {
		prec = 9,
		assoc = LEFT_ASSOCIATIVE,
	},
}
_MODULE.BITOPS = BITOPS
local BITLIB_METHODS = {
	["|"] = "bor",
	["~"] = "bxor",
	["&"] = "band",
	["<<"] = "lshift",
	[">>"] = "rshift",
}
_MODULE.BITLIB_METHODS = BITLIB_METHODS
local BINOPS = {
	["||"] = {
		prec = 3,
		assoc = LEFT_ASSOCIATIVE,
	},
	["&&"] = {
		prec = 4,
		assoc = LEFT_ASSOCIATIVE,
	},
	["=="] = {
		prec = 5,
		assoc = LEFT_ASSOCIATIVE,
	},
	["!="] = {
		prec = 5,
		assoc = LEFT_ASSOCIATIVE,
	},
	["<="] = {
		prec = 5,
		assoc = LEFT_ASSOCIATIVE,
	},
	[">="] = {
		prec = 5,
		assoc = LEFT_ASSOCIATIVE,
	},
	["<"] = {
		prec = 5,
		assoc = LEFT_ASSOCIATIVE,
	},
	[">"] = {
		prec = 5,
		assoc = LEFT_ASSOCIATIVE,
	},
	[".."] = {
		prec = 10,
		assoc = LEFT_ASSOCIATIVE,
	},
	["+"] = {
		prec = 11,
		assoc = LEFT_ASSOCIATIVE,
	},
	["-"] = {
		prec = 11,
		assoc = LEFT_ASSOCIATIVE,
	},
	["*"] = {
		prec = 12,
		assoc = LEFT_ASSOCIATIVE,
	},
	["/"] = {
		prec = 12,
		assoc = LEFT_ASSOCIATIVE,
	},
	["//"] = {
		prec = 12,
		assoc = LEFT_ASSOCIATIVE,
	},
	["%"] = {
		prec = 12,
		assoc = LEFT_ASSOCIATIVE,
	},
	["^"] = {
		prec = 14,
		assoc = RIGHT_ASSOCIATIVE,
	},
}
_MODULE.BINOPS = BINOPS
for token, op in pairs(BITOPS) do
	BINOPS[token] = op
end
for token, op in pairs(BINOPS) do
	op.token = token
end
local BINOP_ASSIGNMENT_TOKENS = {
	["||"] = true,
	["&&"] = true,
	[".."] = true,
	["+"] = true,
	["-"] = true,
	["*"] = true,
	["/"] = true,
	["//"] = true,
	["%"] = true,
	["^"] = true,
	["|"] = true,
	["~"] = true,
	["&"] = true,
	["<<"] = true,
	[">>"] = true,
}
_MODULE.BINOP_ASSIGNMENT_TOKENS = BINOP_ASSIGNMENT_TOKENS
local SURROUND_ENDS = {
	["("] = ")",
	["["] = "]",
	["{"] = "}",
}
_MODULE.SURROUND_ENDS = SURROUND_ENDS
local SYMBOLS = {
	["->"] = true,
	["=>"] = true,
	["..."] = true,
	["::"] = true,
}
_MODULE.SYMBOLS = SYMBOLS
for token, op in pairs(BINOPS) do
	if #token > 1 then
		SYMBOLS[token] = true
	end
end
local STANDARD_ESCAPE_CHARS = {
	a = true,
	b = true,
	f = true,
	n = true,
	r = true,
	t = true,
	v = true,
	["\\"] = true,
	['"'] = true,
	["'"] = true,
	["\n"] = true,
}
_MODULE.STANDARD_ESCAPE_CHARS = STANDARD_ESCAPE_CHARS
local DIGIT = {}
_MODULE.DIGIT = DIGIT
local HEX = {}
_MODULE.HEX = HEX
local WORD_HEAD = {
	["_"] = true,
}
_MODULE.WORD_HEAD = WORD_HEAD
local WORD_BODY = {
	["_"] = true,
}
_MODULE.WORD_BODY = WORD_BODY
for byte = string.byte("0"), string.byte("9") do
	local char = string.char(byte)
	DIGIT[char] = true
	HEX[char] = true
	WORD_BODY[char] = true
end
for byte = string.byte("A"), string.byte("F") do
	local char = string.char(byte)
	HEX[char] = true
	WORD_HEAD[char] = true
	WORD_BODY[char] = true
end
for byte = string.byte("G"), string.byte("Z") do
	local char = string.char(byte)
	WORD_HEAD[char] = true
	WORD_BODY[char] = true
end
for byte = string.byte("a"), string.byte("f") do
	local char = string.char(byte)
	HEX[char] = true
	WORD_HEAD[char] = true
	WORD_BODY[char] = true
end
for byte = string.byte("g"), string.byte("z") do
	local char = string.char(byte)
	WORD_HEAD[char] = true
	WORD_BODY[char] = true
end
return _MODULE
-- Compiled with Erde 0.6.0-1
-- __ERDE_COMPILED__

end,

["erde.init"] = function()
--------------------
-- Module: 'erde.init'
--------------------
local lib = require("erde.lib")
local VERSION
do
	local __ERDE_TMP_4__
	__ERDE_TMP_4__ = require("erde.constants")
	VERSION = __ERDE_TMP_4__["VERSION"]
end
return {
	version = VERSION,
	compile = require("erde.compile"),
	rewrite = lib.rewrite,
	traceback = lib.traceback,
	run = lib.run,
	load = lib.load,
	unload = lib.unload,
}
-- Compiled with Erde 0.6.0-1
-- __ERDE_COMPILED__

end,

["erde.stdlib"] = function()
--------------------
-- Module: 'erde.stdlib'
--------------------
local _MODULE = {}
local _native_coroutine = coroutine
local coroutine = {}
_MODULE.coroutine = coroutine
local _native_debug = debug
local debug = {}
_MODULE.debug = debug
local _native_io = io
local io = {}
_MODULE.io = io
local _native_math = math
local math = {}
_MODULE.math = math
local _native_os = os
local os = {}
_MODULE.os = os
local _native_package = package
local package = {}
_MODULE.package = package
local _native_string = string
local string = {}
_MODULE.string = string
local _native_table = table
local table = {}
_MODULE.table = table
local function load()
	for key, value in pairs(_MODULE) do
		local value_type = type(value)
		if value_type == "function" then
			if key ~= "load" and key ~= "unload" then
				_G[key] = value
			end
		elseif value_type == "table" then
			local library = _G[key]
			if type(library) == "table" then
				for subkey, subvalue in pairs(value) do
					library[subkey] = subvalue
				end
			end
		end
	end
end
_MODULE.load = load
local function unload()
	for key, value in pairs(_MODULE) do
		local value_type = type(value)
		if value_type == "function" then
			if _G[key] == value then
				_G[key] = nil
			end
		elseif value_type == "table" then
			local library = _G[key]
			if type(library) == "table" then
				for subkey, subvalue in pairs(value) do
					if library[subkey] == subvalue then
						library[subkey] = nil
					end
				end
			end
		end
	end
end
_MODULE.unload = unload
local function _kpairs_iter(a, i)
	local key, value = i, nil
	repeat
		key, value = next(a, key)
	until type(key) ~= "number"
	return key, value
end
local function kpairs(t)
	return _kpairs_iter, t, nil
end
_MODULE.kpairs = kpairs
function io.exists(path)
	local file = io.open(path, "r")
	if file == nil then
		return false
	end
	file:close()
	return true
end
function io.readfile(path)
	local file = assert(io.open(path, "r"))
	local content = assert(file:read("*a"))
	file:close()
	return content
end
function io.writefile(path, content)
	local file = assert(io.open(path, "w"))
	assert(file:write(content))
	file:close()
end
function math.clamp(x, min, max)
	return math.min(math.max(x, min), max)
end
function math.round(x)
	if x < 0 then
		return math.ceil(x - 0.5)
	else
		return math.floor(x + 0.5)
	end
end
function math.sign(x)
	if x < 0 then
		return -1
	elseif x > 0 then
		return 1
	else
		return 0
	end
end
function os.capture(cmd)
	local file = assert(io.popen(cmd, "r"))
	local stdout = assert(file:read("*a"))
	file:close()
	return stdout
end
function package.cinsert(...)
	local templates = package.split(package.cpath)
	table.insert(templates, ...)
	package.cpath = package.concat(templates)
end
function package.concat(templates, i, j)
	local template_separator = string.split(package.config, "\n")[2]
	return table.concat(templates, template_separator, i, j)
end
function package.cremove(position)
	local templates = package.split(package.cpath)
	local removed = table.remove(templates, position)
	package.cpath = package.concat(templates)
	return removed
end
function package.insert(...)
	local templates = package.split(package.path)
	table.insert(templates, ...)
	package.path = package.concat(templates)
end
function package.remove(position)
	local templates = package.split(package.path)
	local removed = table.remove(templates, position)
	package.path = package.concat(templates)
	return removed
end
function package.split(path)
	local template_separator = string.split(package.config, "\n")[2]
	return string.split(path, template_separator)
end
local function _string_chars_iter(a, i)
	i = i + 1
	local char = a:sub(i, i)
	if char ~= "" then
		return i, char
	end
end
function string.chars(s)
	return _string_chars_iter, s, 0
end
function string.escape(s)
	local escaped = s:gsub("[().%%+%-*?[^$]", "%%%1")
	return escaped
end
function string.lpad(s, length, padding)
	if padding == nil then
		padding = " "
	end
	return padding:rep(math.ceil((length - #s) / #padding)) .. s
end
function string.ltrim(s, pattern)
	if pattern == nil then
		pattern = "%s+"
	end
	local trimmed = s:gsub(("^" .. tostring(pattern)), "")
	return trimmed
end
function string.pad(s, length, padding)
	if padding == nil then
		padding = " "
	end
	local num_pads = math.ceil(((length - #s) / #padding) / 2)
	return padding:rep(num_pads) .. s .. padding:rep(num_pads)
end
function string.rpad(s, length, padding)
	if padding == nil then
		padding = " "
	end
	return s .. padding:rep(math.ceil((length - #s) / #padding))
end
function string.rtrim(s, pattern)
	if pattern == nil then
		pattern = "%s+"
	end
	local trimmed = s:gsub((tostring(pattern) .. "$"), "")
	return trimmed
end
function string.split(s, separator)
	if separator == nil then
		separator = "%s+"
	end
	local result = {}
	local i, j = s:find(separator)
	while i ~= nil do
		table.insert(result, s:sub(1, i - 1))
		s = s:sub(j + 1) or ""
		i, j = s:find(separator)
	end
	table.insert(result, s)
	return result
end
function string.trim(s, pattern)
	if pattern == nil then
		pattern = "%s+"
	end
	return string.ltrim(string.rtrim(s, pattern), pattern)
end
if _VERSION == "Lua 5.1" then
	table.pack = function(...)
		return {
			n = select("#", ...),
			...,
		}
	end
	table.unpack = unpack
end
function table.clear(t, callback)
	if type(callback) == "function" then
		for key, value in kpairs(t) do
			if callback(value, key) then
				t[key] = nil
			end
		end
		for i = #t, 1, -1 do
			if callback(t[i], i) then
				table.remove(t, i)
			end
		end
	else
		for key, value in kpairs(t) do
			if value == callback then
				t[key] = nil
			end
		end
		for i = #t, 1, -1 do
			if t[i] == callback then
				table.remove(t, i)
			end
		end
	end
end
function table.collect(...)
	local result = {}
	for key, value in ... do
		if value == nil then
			table.insert(result, key)
		else
			result[key] = value
		end
	end
	return result
end
function table.deepcopy(t)
	local result = {}
	for key, value in pairs(t) do
		if type(value) == "table" then
			result[key] = table.deepcopy(value)
		else
			result[key] = value
		end
	end
	return result
end
function table.empty(t)
	return next(t) == nil
end
function table.filter(t, callback)
	local result = {}
	for key, value in pairs(t) do
		if callback(value, key) then
			if type(key) == "number" then
				table.insert(result, value)
			else
				result[key] = value
			end
		end
	end
	return result
end
function table.find(t, callback)
	if type(callback) == "function" then
		for key, value in pairs(t) do
			if callback(value, key) then
				return value, key
			end
		end
	else
		for key, value in pairs(t) do
			if value == callback then
				return value, key
			end
		end
	end
end
function table.has(t, callback)
	local _, key = table.find(t, callback)
	return key ~= nil
end
function table.keys(t)
	local result = {}
	for key, value in pairs(t) do
		table.insert(result, key)
	end
	return result
end
function table.map(t, callback)
	local result = {}
	for key, value in pairs(t) do
		local newValue, newKey = callback(value, key)
		if newKey ~= nil then
			result[newKey] = newValue
		elseif type(key) == "number" then
			table.insert(result, newValue)
		else
			result[key] = newValue
		end
	end
	return result
end
function table.merge(t, ...)
	for _, _t in pairs({
		...,
	}) do
		for key, value in pairs(_t) do
			if type(key) == "number" then
				table.insert(t, value)
			else
				t[key] = value
			end
		end
	end
end
function table.reduce(t, initial, callback)
	local result = initial
	for key, value in pairs(t) do
		result = callback(result, value, key)
	end
	return result
end
function table.reverse(t)
	local len = #t
	for i = 1, math.floor(len / 2) do
		t[i], t[len - i + 1] = t[len - i + 1], t[i]
	end
end
function table.shallowcopy(t)
	local result = {}
	for key, value in pairs(t) do
		result[key] = value
	end
	return result
end
function table.slice(t, i, j)
	if i == nil then
		i = 1
	end
	if j == nil then
		j = #t
	end
	local result, len = {}, #t
	if i < 0 then
		i = i + len + 1
	end
	if j < 0 then
		j = j + len + 1
	end
	for i = math.max(i, 0), math.min(j, len) do
		table.insert(result, t[i])
	end
	return result
end
function table.values(t)
	local result = {}
	for key, value in pairs(t) do
		table.insert(result, value)
	end
	return result
end
setmetatable(coroutine, {
	__index = _native_coroutine,
	__newindex = _native_coroutine,
})
setmetatable(debug, {
	__index = _native_debug,
	__newindex = _native_debug,
})
setmetatable(io, {
	__index = _native_io,
	__newindex = _native_io,
})
setmetatable(math, {
	__index = _native_math,
	__newindex = _native_math,
})
setmetatable(os, {
	__index = _native_os,
	__newindex = _native_os,
})
setmetatable(package, {
	__index = _native_package,
	__newindex = _native_package,
})
setmetatable(string, {
	__index = _native_string,
	__newindex = _native_string,
})
setmetatable(table, {
	__index = _native_table,
	__newindex = _native_table,
})
return _MODULE
-- Compiled with Erde 0.6.0-1
-- __ERDE_COMPILED__

end,

["erde.tokenize"] = function()
--------------------
-- Module: 'erde.tokenize'
--------------------
local config = require("erde.config")
local DIGIT, HEX, STANDARD_ESCAPE_CHARS, SYMBOLS, TOKEN_TYPES, WORD_BODY, WORD_HEAD
do
	local __ERDE_TMP_4__
	__ERDE_TMP_4__ = require("erde.constants")
	DIGIT = __ERDE_TMP_4__["DIGIT"]
	HEX = __ERDE_TMP_4__["HEX"]
	STANDARD_ESCAPE_CHARS = __ERDE_TMP_4__["STANDARD_ESCAPE_CHARS"]
	SYMBOLS = __ERDE_TMP_4__["SYMBOLS"]
	TOKEN_TYPES = __ERDE_TMP_4__["TOKEN_TYPES"]
	WORD_BODY = __ERDE_TMP_4__["WORD_BODY"]
	WORD_HEAD = __ERDE_TMP_4__["WORD_HEAD"]
end
local get_source_alias
do
	local __ERDE_TMP_7__
	__ERDE_TMP_7__ = require("erde.utils")
	get_source_alias = __ERDE_TMP_7__["get_source_alias"]
end
local tokenize_token
local tokens = {}
local text = ""
local current_char = ""
local current_char_index = 1
local current_line = 1
local source_name = ""
local function peek(n)
	return text:sub(current_char_index, current_char_index + n - 1)
end
local function look_ahead(n)
	return text:sub(current_char_index + n, current_char_index + n)
end
local function throw(message, line)
	if line == nil then
		line = current_line
	end
	error((tostring(source_name) .. ":" .. tostring(line) .. ": " .. tostring(message)), 0)
end
local function consume(n)
	if n == nil then
		n = 1
	end
	local consumed = n == 1 and current_char or peek(n)
	current_char_index = current_char_index + n
	current_char = text:sub(current_char_index, current_char_index)
	return consumed
end
local function newline()
	current_line = current_line + 1
	return consume()
end
local function tokenize_binary()
	consume(2)
	if current_char ~= "0" and current_char ~= "1" then
		throw("malformed binary")
	end
	local value = 0
	repeat
		value = 2 * value + tonumber(consume())
	until current_char ~= "0" and current_char ~= "1"
	table.insert(tokens, {
		type = TOKEN_TYPES.NUMBER,
		line = current_line,
		value = tostring(value),
	})
end
local function tokenize_decimal()
	local value = ""
	while DIGIT[current_char] do
		value = value .. consume()
	end
	if current_char == "." and DIGIT[look_ahead(1)] then
		value = value .. consume(2)
		while DIGIT[current_char] do
			value = value .. consume()
		end
	end
	if current_char == "e" or current_char == "E" then
		value = value .. consume()
		if current_char == "+" or current_char == "-" then
			value = value .. consume()
		end
		if not DIGIT[current_char] then
			throw("missing exponent value")
		end
		while DIGIT[current_char] do
			value = value .. consume()
		end
	end
	table.insert(tokens, {
		type = TOKEN_TYPES.NUMBER,
		line = current_line,
		value = value,
	})
end
local function tokenize_hex()
	consume(2)
	if not HEX[current_char] and not (current_char == "." and HEX[look_ahead(1)]) then
		throw("malformed hex")
	end
	local value = 0
	while HEX[current_char] do
		value = 16 * value + tonumber(consume(), 16)
	end
	if current_char == "." and HEX[look_ahead(1)] then
		consume()
		local counter = 1
		repeat
			value = value + tonumber(consume(), 16) / (16 ^ counter)
			counter = counter + 1
		until not HEX[current_char]
	end
	if current_char == "p" or current_char == "P" then
		consume()
		local exponent, sign = 0, 1
		if current_char == "+" or current_char == "-" then
			sign = sign * tonumber(consume() .. "1")
		end
		if not DIGIT[current_char] then
			throw("missing exponent value")
		end
		repeat
			exponent = 10 * exponent + tonumber(consume())
		until not DIGIT[current_char]
		value = value * 2 ^ (sign * exponent)
	end
	table.insert(tokens, {
		type = TOKEN_TYPES.NUMBER,
		line = current_line,
		value = tostring(value),
	})
end
local function escape_sequence()
	if STANDARD_ESCAPE_CHARS[current_char] then
		return consume()
	elseif DIGIT[current_char] then
		return consume()
	elseif current_char == "z" then
		if config.lua_target == "5.1" or config.lua_target == "5.1+" then
			throw("escape sequence \\z not compatible w/ lua targets 5.1, 5.1+")
		end
		return consume()
	elseif current_char == "x" then
		if config.lua_target == "5.1" or config.lua_target == "5.1+" then
			throw("escape sequence \\xXX not compatible w/ lua targets 5.1, 5.1+")
		end
		if not HEX[look_ahead(1)] or not HEX[look_ahead(2)] then
			throw("escape sequence \\xXX must use exactly 2 hex characters")
		end
		return consume(3)
	elseif current_char == "u" then
		if
			config.lua_target == "5.1"
			or config.lua_target == "5.1+"
			or config.lua_target == "5.2"
			or config.lua_target == "5.2+"
		then
			throw("escape sequence \\u{XXX} not compatible w/ lua targets 5.1, 5.1+, 5.2, 5.2+")
		end
		local sequence = consume()
		if current_char ~= "{" then
			throw("missing { in escape sequence \\u{XXX}")
		end
		sequence = sequence .. consume()
		if not HEX[current_char] then
			throw("missing hex in escape sequence \\u{XXX}")
		end
		while HEX[current_char] do
			sequence = sequence .. consume()
		end
		if current_char ~= "}" then
			throw("missing } in escape sequence \\u{XXX}")
		end
		return sequence .. consume()
	else
		throw(("invalid escape sequence \\" .. tostring(current_char)))
	end
end
local function tokenize_interpolation()
	table.insert(tokens, {
		type = TOKEN_TYPES.INTERPOLATION,
		line = current_line,
		value = consume(),
	})
	local interpolation_line = current_line
	local brace_depth = 0
	while current_char ~= "}" or brace_depth > 0 do
		if current_char == "{" then
			brace_depth = brace_depth + 1
			table.insert(tokens, {
				type = TOKEN_TYPES.SYMBOL,
				line = current_line,
				value = consume(),
			})
		elseif current_char == "}" then
			brace_depth = brace_depth - 1
			table.insert(tokens, {
				type = TOKEN_TYPES.SYMBOL,
				line = current_line,
				value = consume(),
			})
		elseif current_char == "" then
			throw("unterminated interpolation", interpolation_line)
		else
			tokenize_token()
		end
	end
	table.insert(tokens, {
		type = TOKEN_TYPES.INTERPOLATION,
		line = current_line,
		value = consume(),
	})
end
local function tokenize_single_quote_string()
	table.insert(tokens, {
		type = TOKEN_TYPES.SINGLE_QUOTE_STRING,
		line = current_line,
		value = consume(),
	})
	local content = ""
	while current_char ~= "'" do
		if current_char == "" or current_char == "\n" then
			throw("unterminated string")
		elseif current_char == "\\" then
			content = content .. consume() .. escape_sequence()
		else
			content = content .. consume()
		end
	end
	if content ~= "" then
		table.insert(tokens, {
			type = TOKEN_TYPES.STRING_CONTENT,
			line = current_line,
			value = content,
		})
	end
	table.insert(tokens, {
		type = TOKEN_TYPES.SINGLE_QUOTE_STRING,
		line = current_line,
		value = consume(),
	})
end
local function tokenize_double_quote_string()
	table.insert(tokens, {
		type = TOKEN_TYPES.DOUBLE_QUOTE_STRING,
		line = current_line,
		value = consume(),
	})
	local content, content_line = "", current_line
	while current_char ~= '"' do
		if current_char == "" or current_char == "\n" then
			throw("unterminated string")
		elseif current_char == "\\" then
			consume()
			if current_char == "{" or current_char == "}" then
				content = content .. consume()
			else
				content = content .. "\\" .. escape_sequence()
			end
		elseif current_char == "{" then
			if content ~= "" then
				table.insert(tokens, {
					type = TOKEN_TYPES.STRING_CONTENT,
					line = content_line,
					value = content,
				})
				content, content_line = "", current_line
			end
			tokenize_interpolation()
		else
			content = content .. consume()
		end
	end
	if content ~= "" then
		table.insert(tokens, {
			type = TOKEN_TYPES.STRING_CONTENT,
			line = content_line,
			value = content,
		})
	end
	table.insert(tokens, {
		type = TOKEN_TYPES.DOUBLE_QUOTE_STRING,
		line = current_line,
		value = consume(),
	})
end
local function tokenize_block_string()
	consume()
	local equals = ""
	while current_char == "=" do
		equals = equals .. consume()
	end
	if current_char ~= "[" then
		throw("unterminated block string opening", current_line)
	end
	consume()
	table.insert(tokens, {
		type = TOKEN_TYPES.BLOCK_STRING,
		line = current_line,
		value = "[" .. equals .. "[",
		equals = equals,
	})
	local close_quote = "]" .. equals .. "]"
	local close_quote_len = #close_quote
	local block_string_line = current_line
	local content, content_line = "", current_line
	while current_char ~= "]" or peek(close_quote_len) ~= close_quote do
		if current_char == "" then
			throw("unterminated block string", block_string_line)
		elseif current_char == "\n" then
			content = content .. newline()
		elseif current_char == "\\" then
			consume()
			if current_char == "{" or current_char == "}" then
				content = content .. consume()
			else
				content = content .. "\\"
			end
		elseif current_char == "{" then
			if content ~= "" then
				table.insert(tokens, {
					type = TOKEN_TYPES.STRING_CONTENT,
					line = content_line,
					value = content,
				})
				content, content_line = "", current_line
			end
			tokenize_interpolation()
		else
			content = content .. consume()
		end
	end
	if content ~= "" then
		table.insert(tokens, {
			type = TOKEN_TYPES.STRING_CONTENT,
			line = content_line,
			value = content,
		})
	end
	table.insert(tokens, {
		type = TOKEN_TYPES.BLOCK_STRING,
		line = current_line,
		value = consume(close_quote_len),
	})
end
local function tokenize_word()
	local word = consume()
	while WORD_BODY[current_char] do
		word = word .. consume()
	end
	table.insert(tokens, {
		type = TOKEN_TYPES.WORD,
		line = current_line,
		value = word,
	})
end
local function tokenize_comment()
	consume(2)
	local is_block_comment, equals = false, ""
	if current_char == "[" then
		consume()
		while current_char == "=" do
			equals = equals .. consume()
		end
		if current_char == "[" then
			consume()
			is_block_comment = true
		end
	end
	if not is_block_comment then
		while current_char ~= "" and current_char ~= "\n" do
			consume()
		end
	else
		local close_quote = "]" .. equals .. "]"
		local close_quote_len = #close_quote
		local comment_line = current_line
		while current_char ~= "]" or peek(close_quote_len) ~= close_quote do
			if current_char == "" then
				throw("unterminated comment", comment_line)
			elseif current_char == "\n" then
				newline()
			else
				consume()
			end
		end
		consume(close_quote_len)
	end
end
function tokenize_token()
	if current_char == "\n" then
		newline()
	elseif current_char == " " or current_char == "\t" then
		consume()
	elseif WORD_HEAD[current_char] then
		tokenize_word()
	elseif current_char == "'" then
		tokenize_single_quote_string()
	elseif current_char == '"' then
		tokenize_double_quote_string()
	else
		local peek_two = peek(2)
		if peek_two == "--" then
			tokenize_comment()
		elseif peek_two == "0x" or peek_two == "0X" then
			tokenize_hex()
		elseif peek_two == "0b" or peek_two == "0B" then
			tokenize_binary()
		elseif DIGIT[current_char] or (current_char == "." and DIGIT[look_ahead(1)]) then
			tokenize_decimal()
		elseif peek_two == "[[" or peek_two == "[=" then
			tokenize_block_string()
		elseif SYMBOLS[peek(3)] then
			table.insert(tokens, {
				type = TOKEN_TYPES.SYMBOL,
				line = current_line,
				value = consume(3),
			})
		elseif SYMBOLS[peek_two] then
			table.insert(tokens, {
				type = TOKEN_TYPES.SYMBOL,
				line = current_line,
				value = consume(2),
			})
		else
			table.insert(tokens, {
				type = TOKEN_TYPES.SYMBOL,
				line = current_line,
				value = consume(1),
			})
		end
	end
end
return function(new_text, new_source_name)
	tokens = {}
	text = new_text
	current_char = text:sub(1, 1)
	current_char_index = 1
	current_line = 1
	source_name = new_source_name or get_source_alias(text)
	if peek(2) == "#!" then
		local shebang = consume(2)
		while current_char ~= "" and current_char ~= "\n" do
			shebang = shebang .. consume()
		end
		table.insert(tokens, {
			type = TOKEN_TYPES.SHEBANG,
			line = current_line,
			value = shebang,
		})
	end
	while current_char ~= "" do
		tokenize_token()
	end
	table.insert(tokens, {
		type = TOKEN_TYPES.EOF,
		line = current_line,
		value = nil,
	})
	return tokens
end
-- Compiled with Erde 0.6.0-1
-- __ERDE_COMPILED__

end,

["erde.utils"] = function()
--------------------
-- Module: 'erde.utils'
--------------------
local _MODULE = {}
local PATH_SEPARATOR
do
	local __ERDE_TMP_2__
	__ERDE_TMP_2__ = require("erde.constants")
	PATH_SEPARATOR = __ERDE_TMP_2__["PATH_SEPARATOR"]
end
local io, string
do
	local __ERDE_TMP_5__
	__ERDE_TMP_5__ = require("erde.stdlib")
	io = __ERDE_TMP_5__["io"]
	string = __ERDE_TMP_5__["string"]
end
local function echo(...)
	return ...
end
_MODULE.echo = echo
local function join_paths(...)
	local joined = table.concat({
		...,
	}, PATH_SEPARATOR):gsub(PATH_SEPARATOR .. "+", PATH_SEPARATOR)
	return joined
end
_MODULE.join_paths = join_paths
local function get_source_summary(source)
	local summary = string.trim(source):sub(1, 5)
	if #source > 5 then
		summary = summary .. "..."
	end
	return summary
end
_MODULE.get_source_summary = get_source_summary
local function get_source_alias(source)
	return ('[string "' .. tostring(get_source_summary(source)) .. '"]')
end
_MODULE.get_source_alias = get_source_alias
return _MODULE
-- Compiled with Erde 0.6.0-1
-- __ERDE_COMPILED__

end,

----------------------
-- Modules part end --
----------------------
        }
        if files[path] then
            return files[path]
        else
            return origin_seacher(path)
        end
    end
end
---------------------------------------------------------
----------------Auto generated code block----------------
---------------------------------------------------------
local lib = require("erde.lib")
local VERSION
do
	local __ERDE_TMP_4__
	__ERDE_TMP_4__ = require("erde.constants")
	VERSION = __ERDE_TMP_4__["VERSION"]
end
return {
	version = VERSION,
	compile = require("erde.compile"),
	rewrite = lib.rewrite,
	traceback = lib.traceback,
	run = lib.run,
	load = lib.load,
	unload = lib.unload,
}
-- Compiled with Erde 0.6.0-1
-- __ERDE_COMPILED__