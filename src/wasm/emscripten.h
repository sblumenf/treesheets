#pragma once

using em_callback_func = void(*)();

inline void emscripten_set_main_loop(em_callback_func func, int fps, int simulate_infinite_loop) {
    // In headless mock, we run the loop a few times to verify logic flow
    for(int i=0; i<3; i++) func();
}
