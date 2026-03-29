// Graphviz config.h — stripped for RustUML vendored build.
// Only layout functionality is needed; all rendering/image library
// dependencies are disabled.

// Include headers
/* #undef HAVE_SYS_INOTIFY_H */
#define HAVE_SYS_IOCTL_H
#define HAVE_SYS_MMAN_H
#define HAVE_SYS_SELECT_H
#define HAVE_SYS_TIME_H
#define HAVE_GETOPT_H

// Functions
/* #undef HAVE_DL_ITERATE_PHDR */
#define HAVE_DRAND48
/* #undef HAVE_INOTIFY_INIT1 */
/* #undef HAVE_MEMRCHR */
/* #undef HAVE_PANGO_FC_FONT_LOCK_FACE */
#define HAVE_SETENV
#define HAVE_SETMODE
#define HAVE_SRAND48
#define HAVE_STRCASESTR

// Typedefs for missing types
#ifdef _MSC_VER
#include <BaseTsd.h>
typedef SSIZE_T ssize_t;
#endif

// Libraries — all disabled for layout-only build
/* #undef HAVE_DEVIL */
/* #undef HAVE_EXPAT */
/* #undef HAVE_FREETYPE */
/* #undef HAVE_LIBGD */
/* #undef HAVE_GD_PNG */
/* #undef HAVE_GD_JPEG */
/* #undef HAVE_GD_XPM */
/* #undef HAVE_GD_FONTCONFIG */
/* #undef HAVE_GD_FREETYPE */
/* #undef HAVE_LASI */
/* #undef HAVE_LIBZ */
/* #undef HAVE_GTS */
/* #undef HAVE_PANGOCAIRO */
/* #undef HAVE_POPPLER */
/* #undef HAVE_QUARTZ */
/* #undef HAVE_RSVG */
/* #undef HAVE_WEBP */

// Values
#define BROWSER "open"
#define DEFAULT_DPI 96
#define GVPLUGIN_CONFIG_FILE "config8"
#define PACKAGE_VERSION "14.1.5~dev.20260326.0520"

// Conditional values
#ifdef __APPLE__
#define DARWIN
/* #undef DARWIN_DYLIB */
#endif
