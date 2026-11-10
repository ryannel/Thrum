// The SDK macros own the signpost wire encoding. The Rust caller supplies only
// a stage number, on the AppKit thread, once per boundary of a qualification run.
#include <dispatch/dispatch.h>
#include <os/signpost.h>

static os_log_t qualification_log;
static dispatch_once_t qualification_log_once;

static void create_qualification_log(void *context) {
    (void)context;
    qualification_log = os_log_create("org.thrum.mac-m2", OS_LOG_CATEGORY_POINTS_OF_INTEREST);
}

void thrum_mac_qualification_signpost(unsigned int stage) {
    dispatch_once_f(&qualification_log_once, NULL, create_qualification_log);
    switch (stage) {
        case 0:
            os_signpost_interval_begin(qualification_log, 1, "MacM2Whole");
            break;
        case 1:
            os_signpost_interval_begin(qualification_log, 2, "MacM2Pan");
            break;
        case 2:
            os_signpost_interval_end(qualification_log, 2, "MacM2Pan");
            break;
        case 3:
            os_signpost_interval_end(qualification_log, 1, "MacM2Whole");
            break;
        default:
            break;
    }
}
