#include <include/base/cef_bind.h>
#include <include/cef_origin_whitelist.h>
#include <include/wrapper/cef_closure_task.h>

#include "app.hh"

#if defined(_WIN64) || defined(_WIN32)
int APIENTRY wWinMain(HINSTANCE hInstance,
                      HINSTANCE hPrevInstance,
                      LPTSTR lpCmdLine,
                      int nCmdShow) {
  CefMainArgs main_args(hInstance);
#else
int main(int argc, char* argv[]) {
  CefMainArgs main_args(argc, argv);
#endif

  // rust_print("cef_interface_execute_process");

  CefRefPtr<CefApp> app(new MyApp({}));

  // CEF applications have multiple sub-processes (render, plugin, GPU, etc)
  // that share the same executable. This function checks the command-line and,
  // if this is a sub-process, executes the appropriate logic.
  int exit_code = CefExecuteProcess(main_args, app, nullptr);
  if (exit_code >= 0) {
    // The sub-process has completed so return here.
    // rust_print("cef_interface_execute_process sub-process has completed");
    return exit_code;
  } else {
    // rust_print("cef_interface_execute_process ???");
    return 0;
  }
}

extern "C" void rust_print(const char* c_str) {
  //
}
