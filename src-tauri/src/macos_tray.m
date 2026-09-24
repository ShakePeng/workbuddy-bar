#import <AppKit/AppKit.h>

static const CGFloat WorkBuddyTitleFontSize = 8.0;

@interface WorkBuddyTrayTextView : NSView
@property (nonatomic, strong) NSTextField *firstLabel;
@property (nonatomic, strong) NSTextField *secondLabel;
@end

@implementation WorkBuddyTrayTextView

- (instancetype)initWithFrame:(NSRect)frameRect {
    self = [super initWithFrame:frameRect];
    if (self) {
        _firstLabel = [self makeLabel];
        _secondLabel = [self makeLabel];
        [self addSubview:_firstLabel];
        [self addSubview:_secondLabel];
    }
    return self;
}

- (NSTextField *)makeLabel {
    NSTextField *label = [[NSTextField alloc] initWithFrame:NSZeroRect];
    label.bezeled = NO;
    label.drawsBackground = NO;
    label.editable = NO;
    label.selectable = NO;
    label.alignment = NSTextAlignmentCenter;
    label.font = [NSFont monospacedDigitSystemFontOfSize:WorkBuddyTitleFontSize
                                                   weight:NSFontWeightSemibold];
    label.textColor = NSColor.labelColor;
    label.maximumNumberOfLines = 1;
    return label;
}

- (void)setFirstLine:(NSString *)firstLine secondLine:(NSString *)secondLine {
    self.firstLabel.stringValue = firstLine ?: @"";
    self.secondLabel.stringValue = secondLine ?: @"";
    [self setNeedsLayout:YES];
}

- (void)layout {
    [super layout];

    NSFont *font = self.firstLabel.font;
    CGFloat lineHeight = ceil(fabs(font.ascender) + fabs(font.descender) + font.leading);
    CGFloat totalHeight = lineHeight * 2.0;
    CGFloat originY = floor((NSHeight(self.bounds) - totalHeight) / 2.0);
    CGFloat width = NSWidth(self.bounds);

    // AppKit 默认坐标原点在左下角，因此第一行放在上方。
    self.secondLabel.frame = NSMakeRect(0, originY, width, lineHeight);
    self.firstLabel.frame = NSMakeRect(0, originY + lineHeight, width, lineHeight);
}

- (NSView *)hitTest:(NSPoint)point {
    return nil;
}

@end

static void workbuddy_apply_tray_title(NSView *view, NSString *title) {
    if ([view isKindOfClass:[NSStatusBarButton class]]) {
        NSStatusBarButton *button = (NSStatusBarButton *)view;
        NSArray<NSString *> *lines = [title componentsSeparatedByString:@"\n"];
        NSString *firstLine = lines.count > 0 ? lines[0] : @"";
        NSString *secondLine = lines.count > 1 ? lines[1] : @"";
        NSFont *font = [NSFont monospacedDigitSystemFontOfSize:WorkBuddyTitleFontSize
                                                        weight:NSFontWeightSemibold];
        NSDictionary<NSAttributedStringKey, id> *measureAttributes = @{NSFontAttributeName: font};
        CGFloat textWidth = ceil(MAX([firstLine sizeWithAttributes:measureAttributes].width,
                                     [secondLine sizeWithAttributes:measureAttributes].width));
        CGFloat imageWidth = button.image == nil ? 0.0 : ceil(button.image.size.width);
        CGFloat requiredWidth = imageWidth + textWidth + 8.0;
        if (NSWidth(button.frame) < requiredWidth) {
            NSRect frame = button.frame;
            frame.size.width = requiredWidth;
            button.frame = frame;
        }

        // 透明 attributed title 只负责保留标题布局，实际双行文字由子视图绘制。
        NSDictionary<NSAttributedStringKey, id> *placeholderAttributes = @{
            NSForegroundColorAttributeName: NSColor.clearColor,
            NSFontAttributeName: font,
        };
        button.controlSize = NSControlSizeMini;
        button.usesSingleLineMode = YES;
        button.lineBreakMode = NSLineBreakByClipping;
        button.attributedTitle = [[NSAttributedString alloc] initWithString:secondLine
                                                                  attributes:placeholderAttributes];

        WorkBuddyTrayTextView *textView = nil;
        for (NSView *subview in button.subviews) {
            if ([subview isKindOfClass:[WorkBuddyTrayTextView class]]) {
                textView = (WorkBuddyTrayTextView *)subview;
                break;
            }
        }
        if (textView == nil) {
            textView = [[WorkBuddyTrayTextView alloc] initWithFrame:NSZeroRect];
            textView.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
            [button addSubview:textView];
        }

        NSRect titleRect = [button.cell titleRectForBounds:button.bounds];
        if (NSWidth(titleRect) <= 0.0 || NSHeight(titleRect) <= 0.0) {
            titleRect = button.bounds;
        }
        // 自定义视图使用按钮的完整高度，避免系统单行 titleRect 的高度约束影响双行绘制。
        CGFloat textOriginX = NSMinX(titleRect);
        CGFloat textViewWidth = MAX(0.0, NSWidth(button.bounds) - textOriginX);
        textView.frame = NSMakeRect(textOriginX, 0, textViewWidth, NSHeight(button.bounds));
        [textView setFirstLine:firstLine secondLine:secondLine];
        [textView layoutSubtreeIfNeeded];
        button.appearsDisabled = NO;
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
