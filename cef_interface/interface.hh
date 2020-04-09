typedef void (*OnPaintCallback)(const void* pixels, int width, int height);

extern "C" int cef_init(OnPaintCallback onPaintCallback);
extern "C" int cef_free();
extern "C" int cef_step();
extern "C" int cef_load(const char* url);
