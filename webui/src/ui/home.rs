use common::server::EndpointOS;
use leptos::{component, view, IntoView};
use leptos_meta::Stylesheet;

use crate::components::{footer::Footer, menu::Menu, stats::endpoints::Stats};

#[component]
/// Home page
pub(crate) fn Home() -> impl IntoView {
    let all = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-20 w-20" viewBox="0 0 24 24"><path d="M4,6H20V16H4M20,18A2,2 0 0,0 22,16V6C22,4.89 21.1,4 20,4H4C2.89,4 2,4.89 2,6V16A2,2 0 0,0 4,18H0V20H24V18H20Z" /></svg>"#,
    );
    let windows = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-20 w-20" viewBox="0 0 24 24"><path d="M3,12V6.75L9,5.43V11.91L3,12M20,3V11.75L10,11.9V5.21L20,3M3,13L9,13.09V19.9L3,18.75V13M20,13.25V22L10,20.09V13.1L20,13.25Z" /></svg>"#,
    );
    let darwin = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-20 w-20" viewBox="0 0 24 24"><path d="M18.71,19.5C17.88,20.74 17,21.95 15.66,21.97C14.32,22 13.89,21.18 12.37,21.18C10.84,21.18 10.37,21.95 9.1,22C7.79,22.05 6.8,20.68 5.96,19.47C4.25,17 2.94,12.45 4.7,9.39C5.57,7.87 7.13,6.91 8.82,6.88C10.1,6.86 11.32,7.75 12.11,7.75C12.89,7.75 14.37,6.68 15.92,6.84C16.57,6.87 18.39,7.1 19.56,8.82C19.47,8.88 17.39,10.1 17.41,12.63C17.44,15.65 20.06,16.66 20.09,16.67C20.06,16.74 19.67,18.11 18.71,19.5M13,3.5C13.73,2.67 14.94,2.04 15.94,2C16.07,3.17 15.6,4.35 14.9,5.19C14.21,6.04 13.07,6.7 11.95,6.61C11.8,5.46 12.36,4.26 13,3.5Z" /></svg>"#,
    );
    let linux = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-20 w-20" viewBox="0 0 24 24"><path d="M19,16C19,17.72 18.37,19.3 17.34,20.5C17.75,20.89 18,21.41 18,22H6C6,21.41 6.25,20.89 6.66,20.5C5.63,19.3 5,17.72 5,16H3C3,14.75 3.57,13.64 4.46,12.91L4.47,12.89C6,11.81 7,10 7,8V7A5,5 0 0,1 12,2A5,5 0 0,1 17,7V8C17,10 18,11.81 19.53,12.89L19.54,12.91C20.43,13.64 21,14.75 21,16H19M16,16A4,4 0 0,0 12,12A4,4 0 0,0 8,16A4,4 0 0,0 12,20A4,4 0 0,0 16,16M10,9L12,10.5L14,9L12,7.5L10,9M10,5A1,1 0 0,0 9,6A1,1 0 0,0 10,7A1,1 0 0,0 11,6A1,1 0 0,0 10,5M14,5A1,1 0 0,0 13,6A1,1 0 0,0 14,7A1,1 0 0,0 15,6A1,1 0 0,0 14,5Z" /></svg>"#,
    );

    view! {
        <Stylesheet id="leptos" href="/pkg/tailwind.css"/>
        <div class="grid grid-cols-4">
            <Menu />
            <div class="col-span-1"><Stats os=EndpointOS::All html=all /></div>
            <div class="col-span-1"><Stats os=EndpointOS::Linux html=linux/></div>
            <div class="col-span-1"><Stats os=EndpointOS::Darwin html=darwin/></div>
            <div class="col-span-1"><Stats os=EndpointOS::Windows html=windows/></div>
        </div>
        <Footer />
    }
}
