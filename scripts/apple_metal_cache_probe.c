// Loaded only by the launch witness, before main or device creation.
// Metal's getter attests the system cache used during this process.
#include <CoreFoundation/CoreFoundation.h>
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

static CFStringRef (*get_shader_cache_path)(void);

__attribute__((constructor))
static void select_cache(void) {
    const char *directory = getenv("THRUM_METAL_CACHE_PATH");
    if (!directory)
        return;
    void *metal = dlopen("/System/Library/Frameworks/Metal.framework/Metal", RTLD_NOW);
    if (!metal)
        return;
    void (*set_shader_cache_path)(CFStringRef) = dlsym(metal, "MTLSetShaderCachePath");
    get_shader_cache_path = dlsym(metal, "MTLGetShaderCachePath");
    if (!set_shader_cache_path)
        return;
    CFStringRef path = CFStringCreateWithFileSystemRepresentation(NULL, directory);
    if (path) {
        set_shader_cache_path(path);
        CFRelease(path);
    }
}

__attribute__((destructor))
static void report_cache(void) {
    if (!get_shader_cache_path)
        return;
    CFStringRef path = get_shader_cache_path();
    char bytes[4096];
    if (path && CFStringGetFileSystemRepresentation(path, bytes, sizeof(bytes)))
        fprintf(stderr, "THRUM_METAL_CACHE %s\n", bytes);
}
