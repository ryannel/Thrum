#include <mach/mach_time.h>
#include <os/signpost.h>

static os_log_t log_for_pan(void) {
    // Both calls originate on the one AppKit thread.
    static os_log_t log = NULL;
    if (log == NULL) {
        log = os_log_create("org.thrum.v1", OS_LOG_CATEGORY_POINTS_OF_INTEREST);
    }
    return log;
}

uint64_t thrum_pan_start(void) {
    const uint64_t now = mach_absolute_time();
    os_signpost_event_emit(log_for_pan(), OS_SIGNPOST_ID_EXCLUSIVE, "ThrumM2PanStart");
    return now;
}

uint64_t thrum_pan_end(void) {
    const uint64_t now = mach_absolute_time();
    os_signpost_event_emit(log_for_pan(), OS_SIGNPOST_ID_EXCLUSIVE, "ThrumM2PanEnd");
    return now;
}
