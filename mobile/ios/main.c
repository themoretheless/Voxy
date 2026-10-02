/* Xcode executable main. winit owns UIApplicationMain; do not call it twice. */
extern void voxy_ios_main(void);
int main(void) {
    voxy_ios_main();
    return 0;
}
