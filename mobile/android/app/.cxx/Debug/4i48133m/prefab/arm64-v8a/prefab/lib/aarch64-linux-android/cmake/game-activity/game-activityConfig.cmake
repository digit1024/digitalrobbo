if(NOT TARGET game-activity::game-activity)
add_library(game-activity::game-activity STATIC IMPORTED)
set_target_properties(game-activity::game-activity PROPERTIES
    IMPORTED_LOCATION "/home/digit1024/.gradle/caches/transforms-4/3f0b780ff134fb087028bbce58373f8b/transformed/games-activity-2.0.2/prefab/modules/game-activity/libs/android.arm64-v8a/libgame-activity.a"
    INTERFACE_INCLUDE_DIRECTORIES "/home/digit1024/.gradle/caches/transforms-4/3f0b780ff134fb087028bbce58373f8b/transformed/games-activity-2.0.2/prefab/modules/game-activity/include"
    INTERFACE_LINK_LIBRARIES ""
)
endif()

if(NOT TARGET game-activity::game-activity_static)
add_library(game-activity::game-activity_static STATIC IMPORTED)
set_target_properties(game-activity::game-activity_static PROPERTIES
    IMPORTED_LOCATION "/home/digit1024/.gradle/caches/transforms-4/3f0b780ff134fb087028bbce58373f8b/transformed/games-activity-2.0.2/prefab/modules/game-activity_static/libs/android.arm64-v8a/libgame-activity_static.a"
    INTERFACE_INCLUDE_DIRECTORIES "/home/digit1024/.gradle/caches/transforms-4/3f0b780ff134fb087028bbce58373f8b/transformed/games-activity-2.0.2/prefab/modules/game-activity_static/include"
    INTERFACE_LINK_LIBRARIES ""
)
endif()

