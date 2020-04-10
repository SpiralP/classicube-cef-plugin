typedef void (*OnPaintCallback)(const void* pixels, int width, int height);

extern "C" int cef_init(OnPaintCallback onPaintCallback);
extern "C" int cef_free();
extern "C" int cef_step();
extern "C" int cef_load(const char* url);
extern "C" int cef_run_script(const char* code);

extern "C" void rust_print(const char* c_str);
