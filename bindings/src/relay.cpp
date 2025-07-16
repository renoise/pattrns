/**
 * When enabled (linked into a binary) the pattrns shared lib can be loaded manually at
 * runtime from a specified runtime path.
 *
 * To do so it adds three new functions to the pattrns API: 'load_library(path)', 'unload_library' and
 * 'library_loaded'.
 *
 * Unfortunately the relay isn't auto-generated, so it must be updated manually when the pattrns API
 * changes. When it's used, missing API functions will result in linker errors.
 */

#include "../includes/pattrns.h"

#include <string>
#include <vector>
#include <functional>
#include <stdexcept>
#include <locale>
#include <codecvt>

#if _WIN32
  #define WIN32_LEAN_AND_MEAN
  #include "Windows.h"
#else
  #define DLOPEN_NO_WARN
  #include <string.h>
  #include <dlfcn.h>
#endif

// -------------------------------------------------------------------------------------------------

#define CREATE_FUNCTION_RELAY(NAME) \
  static auto NAME##_relay = PattrnsFunction<decltype(&pattrns::NAME)>(#NAME);

static std::vector<std::function<void(void *)>> function_resolvers = {};
static std::vector<std::function<void()>> function_resetters = {};
static void *library_instance = nullptr;

template <typename FuncType>
class PattrnsFunction
{
public:
  PattrnsFunction(const char *func_name)
    : func_(nullptr)
  {
    function_resolvers.push_back([this, func_name](void *library) {
#if _WIN32
      func_ = (FuncType)::GetProcAddress((HMODULE)library, func_name);
#else
      func_ = (FuncType)::dlsym(library, func_name);
#endif
      if (! func_)
      {
        throw std::runtime_error(
          std::string() + "failed to resolve pattrns function: '" + func_name + "'.");
      }
    });

    function_resetters.push_back([this] { func_ = nullptr; });
  }

  template <class... ArgTypes>
  inline auto operator()(ArgTypes &&...args)
  {
    return (this->func_)(std::forward<ArgTypes>(args)...);
  }

private:
  FuncType func_;
};

// -------------------------------------------------------------------------------------------------

namespace pattrns
{
  // New pattrns relay functions (CPP)

  bool library_loaded()
  {
    return library_instance != nullptr;
  }

  void load_library(
#if _WIN32
    const wchar_t *dllname
#else
    const char *dllname
#endif
  )
  {
#if _WIN32
    library_instance = (void *)::LoadLibraryW(dllname);
#else
    library_instance = (void *)::dlopen(dllname, RTLD_LOCAL | RTLD_NOW);
#endif
    if (! library_instance)
    {
      throw std::runtime_error(
        std::string() + "Failed to load pattrns shared library file. Error: " +
#if defined(_WIN32)
        std::to_string(::GetLastError())
#else
        std::string(::strerror(errno))
#endif
      );
    }

    for (const auto &resolver : function_resolvers)
    {
      resolver(library_instance);
    }
  }

  void unload_library()
  {
#if _WIN32
    ::FreeLibrary((HMODULE)library_instance);
#else
    ::dlclose(library_instance);
#endif

    library_instance = nullptr;
    for (const auto &resetter : function_resetters)
    {
      resetter();
    }
  }

  // pattrns API function impls (raw C)

  CREATE_FUNCTION_RELAY(initialize);
  extern "C" VoidResult initialize(AllocFn alloc, DeallocFn dealloc)
  {
    return initialize_relay(alloc, dealloc);
  }

  CREATE_FUNCTION_RELAY(finalize);
  extern "C" VoidResult finalize()
  {
    return finalize_relay();
  }

  CREATE_FUNCTION_RELAY(drop_error_string);
  extern "C" void drop_error_string(const char *error)
  {
    drop_error_string_relay(error);
  }

  CREATE_FUNCTION_RELAY(drop_parameter_set);
  extern "C" void drop_parameter_set(ParameterSet *parameters)
  {
    drop_parameter_set_relay(parameters);
  }

  CREATE_FUNCTION_RELAY(new_pattern_from_file);
  extern "C" PatternResult new_pattern_from_file(
    Timebase time_base,
    const uint32_t *instrument_id,
    const char *file_name)
  {
    return new_pattern_from_file_relay(time_base, instrument_id, file_name);
  }

  CREATE_FUNCTION_RELAY(new_pattern_from_string);
  extern "C" PatternResult new_pattern_from_string(
    Timebase time_base,
    const uint32_t *instrument_id,
    const char *content,
    const char *content_name)
  {
    return new_pattern_from_string_relay(time_base, instrument_id, content, content_name);
  }

  CREATE_FUNCTION_RELAY(new_pattern_instance);
  extern "C" PatternResult new_pattern_instance(Pattern *pattern, Timebase time_base)
  {
    return new_pattern_instance_relay(pattern, time_base);
  }

  CREATE_FUNCTION_RELAY(drop_pattern);
  extern "C" void drop_pattern(Pattern *pattern)
  {
    drop_pattern_relay(pattern);
  }

  CREATE_FUNCTION_RELAY(pattern_parameters);
  extern "C" ParameterSetResult pattern_parameters(Pattern *pattern)
  {
    return pattern_parameters_relay(pattern);
  }

  CREATE_FUNCTION_RELAY(set_pattern_parameter_value);
  extern "C" VoidResult set_pattern_parameter_value(Pattern *pattern, const char *id, double value)
  {
    return set_pattern_parameter_value_relay(pattern, id, value);
  }

  CREATE_FUNCTION_RELAY(pattern_samples_per_step);
  extern "C" F64Result pattern_samples_per_step(Pattern* pattern)
  {
    return pattern_samples_per_step_relay(pattern);
  }

  CREATE_FUNCTION_RELAY(pattern_step_count);
  extern "C" UInt32Result pattern_step_count(Pattern* pattern)
  {
    return pattern_step_count_relay(pattern);
  }

  CREATE_FUNCTION_RELAY(set_pattern_time_base);
  extern "C" VoidResult set_pattern_time_base(Pattern *pattern, Timebase time_base)
  {
    return set_pattern_time_base_relay(pattern, time_base);
  }

  CREATE_FUNCTION_RELAY(set_pattern_trigger_event);
  extern "C" VoidResult set_pattern_trigger_event(
    Pattern *pattern,
    const NoteEvent *note_events_ptr,
    uint32_t note_events_len)
  {
    return set_pattern_trigger_event_relay(pattern, note_events_ptr, note_events_len);
  }

  CREATE_FUNCTION_RELAY(run_pattern);
  extern "C" VoidResult run_pattern(
    Pattern *pattern,
    void *callback_context,
    void (*callback)(void *, const PatternPlaybackEvent *))
  {
    return run_pattern_relay(pattern, callback_context, callback);
  }

  CREATE_FUNCTION_RELAY(run_pattern_until_time);
  extern "C" VoidResult run_pattern_until_time(
    Pattern *pattern,
    uint64_t time,
    void *callback_context,
    void (*callback)(void *, const PatternPlaybackEvent *))
  {
    return run_pattern_until_time_relay(pattern, time, callback_context, callback);
  }

  CREATE_FUNCTION_RELAY(advance_pattern_until_time);
  extern "C" VoidResult advance_pattern_until_time(Pattern *pattern, uint64_t time)
  {
    return advance_pattern_until_time_relay(pattern, time);
  }
}
