#include <time.h>

struct ScheduledPickup {
    time_t ready;
    unsigned int drinks;
};

static inline double scheduling_wait(struct ScheduledPickup pickup, time_t now) {
    return difftime(pickup.ready, now);
}
