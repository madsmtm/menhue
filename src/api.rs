use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use block2::RcBlock;
use objc2::{rc::Retained, runtime::AnyObject, Message};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSArray, NSCopying, NSData, NSDictionary, NSError,
    NSHTTPURLResponse, NSJSONReadingOptions, NSJSONSerialization, NSJSONWritingOptions,
    NSLocalizedDescriptionKey, NSMutableArray, NSMutableURLRequest, NSNumber, NSOperationQueue,
    NSString, NSURLComponents, NSURLErrorKey, NSURLRequestCachePolicy,
    NSURLRequestNetworkServiceType, NSURLResponse, NSURLSession, NSURLSessionConfiguration,
    NSURLSessionTask,
};

pub const HTTP_STATUS_CODE_DOMAIN: &'static str = "HTTPCodeError";
pub const HUE_API_ERROR: &'static str = "HueAPIError";

#[derive(Debug, Clone)]
pub struct Session {
    url_session: Retained<NSURLSession>,
    host: Rc<RefCell<Option<Retained<NSString>>>>,
    username: Rc<RefCell<Option<Retained<NSString>>>>,
}

impl Session {
    pub fn new(
        _mtm: MainThreadMarker,
        host: Rc<RefCell<Option<Retained<NSString>>>>,
        username: Rc<RefCell<Option<Retained<NSString>>>>,
    ) -> Self {
        let config = NSURLSessionConfiguration::ephemeralSessionConfiguration();
        // It makes no sense to try to configure Hue lights on cellular networks
        config.setAllowsCellularAccess(false);
        // Time out after 5 seconds; the Hue bridge is on the local network
        config.setTimeoutIntervalForRequest(5.0);
        // TODO
        config.setHTTPMaximumConnectionsPerHost(1);
        // We only make requests on direct user action
        config.setNetworkServiceType(
            NSURLRequestNetworkServiceType::NetworkServiceTypeResponsiveData,
        );
        let url_session = unsafe {
            NSURLSession::sessionWithConfiguration_delegate_delegateQueue(
                &config,
                None,
                // Ensure that all operations are executed on the main thread
                Some(&NSOperationQueue::mainQueue()),
            )
        };
        url_session.setSessionDescription(Some(ns_string!("bridge connection")));

        Self {
            url_session,
            host,
            username,
        }
    }

    pub fn request(
        &self,
        method: &NSString,
        path: &NSString,
        json_object: Option<&AnyObject>,
        completion_handler: impl FnOnce(Result<Retained<AnyObject>, Retained<NSError>>) + 'static,
    ) -> Retained<NSURLSessionTask> {
        let components = NSURLComponents::new();
        components.setHost(Some(
            self.host
                .borrow()
                .as_ref()
                .expect("host must be set before making URL request"),
        ));
        components.setPath(Some(path));
        // TODO: Use encryption from API V2
        components.setScheme(Some(ns_string!("http")));
        let url = components.URL().expect("building NSURL from components");

        let body = json_object.map(|json_object| {
            unsafe {
                NSJSONSerialization::dataWithJSONObject_options_error(
                    json_object,
                    NSJSONWritingOptions::PrettyPrinted,
                )
            }
            .expect("json writing")
        });

        let request = NSMutableURLRequest::requestWithURL(&url);
        request.setCachePolicy(NSURLRequestCachePolicy::ReloadIgnoringCacheData);
        request.setHTTPMethod(method);
        request.setHTTPBody(body.as_deref());
        request.addValue_forHTTPHeaderField(
            ns_string!("application/json"),
            ns_string!("Content-Type"),
        );

        let completion_handler = Cell::new(Some(completion_handler));
        let block = RcBlock::new(
            move |body: *mut NSData, response: *mut NSURLResponse, error: *mut NSError| {
                let completion_handler = completion_handler
                    .take()
                    .expect("completion handler called twice");
                if let Some(error) = unsafe { error.as_ref() } {
                    return completion_handler(Err(error.retain()));
                }

                let response =
                    unsafe { response.as_ref() }.expect("response should be set if not an error");
                let body = unsafe { body.as_ref() }.expect("body should be set if not an error");

                let response = response
                    .downcast_ref::<NSHTTPURLResponse>()
                    .expect("invalid kind of NSHTTPURLResponse");
                let status_code = response.statusCode();
                if !(200..300).contains(&status_code) {
                    // TODO: Attempt to parse body here?
                    let dict = NSDictionary::from_retained_objects(
                        unsafe { &[NSURLErrorKey, NSLocalizedDescriptionKey] },
                        &[
                            Retained::into_super(Retained::into_super(url.retain())),
                            Retained::into_super(Retained::into_super(
                                NSHTTPURLResponse::localizedStringForStatusCode(status_code),
                            )),
                        ],
                    );
                    let error = unsafe {
                        NSError::errorWithDomain_code_userInfo(
                            ns_string!(HTTP_STATUS_CODE_DOMAIN),
                            status_code,
                            Some(&*dict),
                        )
                    };
                    return completion_handler(Err(error));
                }

                let json = NSJSONSerialization::JSONObjectWithData_options_error(
                    body,
                    NSJSONReadingOptions::empty(),
                );
                let json = match json {
                    Ok(json) => json,
                    Err(err) => return completion_handler(Err(err)),
                };

                let parse_error = |json: &AnyObject| {
                    let Some(json) = json.downcast_ref::<NSDictionary>() else {
                        let dict = NSDictionary::from_retained_objects(
                            unsafe { &[NSURLErrorKey, NSLocalizedDescriptionKey] },
                            &[
                                Retained::into_super(Retained::into_super(url.retain())),
                                Retained::into_super(Retained::into_super(
                                    ns_string!("invalid error response object").copy(),
                                )),
                            ],
                        );
                        return unsafe {
                            NSError::errorWithDomain_code_userInfo(
                                ns_string!(HUE_API_ERROR),
                                0,
                                Some(&dict),
                            )
                        };
                    };
                    let status_code = json
                        .objectForKey(ns_string!("type"))
                        .and_then(|n| n.downcast::<NSNumber>().ok())
                        .map(|n| n.as_isize())
                        .unwrap_or(0);
                    let description = json
                        .objectForKey(ns_string!("description"))
                        .map(|s| {
                            s.downcast::<NSString>().unwrap_or_else(|_| {
                                ns_string!("incorrect error description type").copy()
                            })
                        })
                        .unwrap_or_else(|| ns_string!("no error description").copy());

                    // TODO: Hue error is not localized
                    let dict = NSDictionary::from_retained_objects(
                        unsafe { &[NSURLErrorKey, NSLocalizedDescriptionKey] },
                        &[
                            Retained::into_super(Retained::into_super(url.retain())),
                            Retained::into_super(Retained::into_super(description.copy())),
                        ],
                    );
                    unsafe {
                        NSError::errorWithDomain_code_userInfo(
                            ns_string!(HUE_API_ERROR),
                            status_code,
                            Some(&dict),
                        )
                    }
                };

                if let Some(array) = json.downcast_ref::<NSArray>() {
                    let result = NSMutableArray::arrayWithCapacity(array.len());

                    // Scan the array for errors
                    for item in array {
                        if let Some(dict) = item.downcast_ref::<NSDictionary>() {
                            if let Some(error) = dict.objectForKey(ns_string!("error")) {
                                return completion_handler(Err(parse_error(&error)));
                            }

                            // Unwrap "success" key, if present
                            if let Some(item) = dict.objectForKey(ns_string!("success")) {
                                result.addObject(&*item);
                                continue;
                            }
                        }

                        result.addObject(&item);
                    }

                    completion_handler(Ok(Retained::into_super(Retained::into_super(
                        Retained::into_super(result),
                    ))))
                } else if let Some(dict) = json.downcast_ref::<NSDictionary>() {
                    if let Some(error) = dict.objectForKey(ns_string!("error")) {
                        return completion_handler(Err(parse_error(&error)));
                    }

                    // Unwrap "success" key, if present
                    if let Some(json) = dict.objectForKey(ns_string!("success")) {
                        return completion_handler(Ok(json));
                    }

                    completion_handler(Ok(json))
                } else {
                    panic!("NSJSONSerialization neither returned an array nor a dictionary")
                }
            },
        );

        let task = unsafe {
            self.url_session
                .dataTaskWithRequest_completionHandler(&request, &block)
        };
        task.resume();
        Retained::into_super(task)
    }

    pub fn connect(
        &self,
        completion_handler: impl FnOnce(Result<(), Retained<NSError>>) + 'static,
    ) -> Retained<NSURLSessionTask> {
        let json = NSDictionary::from_retained_objects(
            &[ns_string!("devicetype")],
            &[ns_string!("test").copy()],
        );
        let username_rc = Rc::clone(&self.username);
        self.request(
            ns_string!("POST"),
            ns_string!("/api"),
            Some(&json),
            move |res| {
                completion_handler(res.map(|obj| {
                    let array = obj
                        .downcast_ref::<NSArray>()
                        .unwrap_or_else(|| todo!("invalid response: {obj:?}"));

                    let dict = array
                        .objectAtIndex(0)
                        .downcast::<NSDictionary>()
                        .unwrap_or_else(|_| todo!("invalid response: {array:?}"));

                    let username = dict
                        .objectForKey(ns_string!("username"))
                        .expect("no username")
                        .downcast::<NSString>()
                        .unwrap_or_else(|_| todo!("invalid username: {dict:?}"));

                    dbg!(&username);

                    *username_rc.borrow_mut() = Some(username);
                }))
            },
        )
    }

    pub fn authenticated_path(&self, path: &str) -> Retained<NSString> {
        let res = NSString::from_str(&format!(
            "/api/{}{path}",
            self.username
                .borrow()
                .as_deref()
                .unwrap_or_else(|| ns_string!("not-set"))
        ));
        res
    }

    pub fn destroy(&self) {
        self.url_session.invalidateAndCancel();
    }
}
