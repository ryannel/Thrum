#include <mach/mach_time.h>
#include <os/signpost.h>
#include <sys/time.h>

struct pan_anchor {
    uint64_t mach_before;
    uint64_t mach_after;
    uint64_t unix_ns;
};

static os_log_t log_for_pan(void) {
    // Both calls originate on the one AppKit thread.
    static os_log_t log = NULL;
    if (log == NULL) {
        log = os_log_create("org.thrum.v1", OS_LOG_CATEGORY_POINTS_OF_INTEREST);
    }
    return log;
}

static struct pan_anchor sample_anchor(void) {
    struct timeval realtime = {0};
    struct pan_anchor anchor = {0};
    anchor.mach_before = mach_absolute_time();
    const int status = gettimeofday(&realtime, NULL);
    anchor.mach_after = mach_absolute_time();
    if (status == 0) {
        anchor.unix_ns = (uint64_t)realtime.tv_sec * UINT64_C(1000000000)
            + (uint64_t)realtime.tv_usec * UINT64_C(1000);
    }
    return anchor;
}

struct pan_anchor thrum_pan_start(void) {
    os_log_t log = log_for_pan();
    const struct pan_anchor anchor = sample_anchor();
    os_signpost_event_emit(log, OS_SIGNPOST_ID_EXCLUSIVE, "ThrumM2PanStart");
    return anchor;
}

struct pan_anchor thrum_pan_end(void) {
    os_log_t log = log_for_pan();
    const struct pan_anchor anchor = sample_anchor();
    os_signpost_event_emit(log, OS_SIGNPOST_ID_EXCLUSIVE, "ThrumM2PanEnd");
    return anchor;
}
