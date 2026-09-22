#import <AppKit/AppKit.h>

static void workbuddy_apply_tray_title(NSView *view, NSString *title) {
    if ([view isKindOfClass:[NSStatusBarButton class]]) {
        NSStatusBarButton *button = (NSStatusBarButton *)view;
        if ([button.title isEqualToString:title]) {
            NSDictionary<NSAttributedStringKey, id> *attributes = @{
                NSForegroundColorAttributeName: NSColor.labelColor,
                NSFontAttributeName: [NSFont systemFontOfSize:NSFont.systemFontSize
                                                       weight:NSFontWeightSemibold],
            };
            button.attributedTitle = [[NSAttributedString alloc] initWithString:title
                                                                      attributes:attributes];
            button.appearsDisabled = NO;
        }
    }

    for (NSView *child in view.subviews) {
        workbuddy_apply_tray_title(child, title);
    }
}

void workbuddy_set_tray_title(const char *title_utf8) {
    if (title_utf8 == NULL) {
        return;
    }

    NSString *title = [NSString stringWithUTF8String:title_utf8];
    if (title.length == 0) {
        return;
    }

    dispatch_async(dispatch_get_main_queue(), ^{
        for (NSWindow *window in NSApp.windows) {
            if (window.contentView != nil) {
                workbuddy_apply_tray_title(window.contentView, title);
            }
        }
    });
}
