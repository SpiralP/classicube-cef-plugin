#include <include/base/cef_bind.h>
#include <include/cef_origin_whitelist.h>
#include <include/wrapper/cef_closure_task.h>
#if defined(OS_MACOSX)
#include <include/wrapper/cef_library_loader.h>
#endif

#include "app.hh"

int cleanup_and_return(int r) {
#if defined(OS_MACOSX)
  if (!cef_unload_library()) {
    return 1;
  }
#endif

  return r;
}

#if defined(_WIN64) || defined(_WIN32)
int APIENTRY wWinMain(HINSTANCE hInstance,
                      HINSTANCE hPrevInstance,
                      LPTSTR lpCmdLine,
                      int nCmdShow) {
  CefMainArgs main_args(hInstance);
#else
int main(int argc, char* argv[]) {

#if defined(OS_MACOSX)
  if (!cef_load_library("./cef/Chromium Embedded Framework.framework/Chromium "
                        "Embedded Framework")) {
    return 1;
  }
#endif

  CefMainArgs main_args(argc, argv);
#endif

  // rust_debug("cef_interface_execute_process");

  CefRefPtr<CefApp> app(new MyApp({}));

  // CEF applications have multiple sub-processes (render, plugin, GPU, etc)
  // that share the same executable. This function checks the command-line and,
  // if this is a sub-process, executes the appropriate logic.
  int exit_code = CefExecuteProcess(main_args, app, nullptr);
  if (exit_code >= 0) {
    // The sub-process has completed so return here.
    // rust_debug("cef_interface_execute_process sub-process has completed");
    return cleanup_and_return(exit_code);
  } else {
    // rust_debug("cef_interface_execute_process ???");
    return cleanup_and_return(0);
  }
}

extern "C" void rust_debug(const char* c_str) {
  printf("DEBUG: %s\n", c_str);
  LOG(INFO) << CefString(c_str);
}

extern "C" void rust_warn(const char* c_str) {
  printf("WARN: %s\n", c_str);
  LOG(WARNING) << CefString(c_str);
}
